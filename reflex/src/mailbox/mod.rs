
use crate::msg_union::{MessageTypeUnion, MailboxEntry};
use crate::internal::MsgQueueEntry;

use futures::prelude::*;
use futures::sync::mpsc;

/// A handle for sending messages to an actor.
///
/// Notable properties include:
/// - this type is `Clone`
/// - this type is a `Sink`
/// - if the actor is dead, sending into the mailbox will silently swallow the error
/// - sending a message to an actor provides no guarantee of either:
///   - delivery (consequential to silently ignoring actor-death)
///   - successful processing
/// - this type has methods which delegate to the `futures::sync::mpsc::Sender` methods:
///   - `try_send` (`Mailbox::send_now`)
///   - `is_closed`
pub struct Mailbox<T: MessageTypeUnion> {
    sender: mpsc::Sender<MsgQueueEntry<T>>,
}

impl<T: MessageTypeUnion> Mailbox<T> {
    /// Crate-internal constructor.
    pub (crate) fn from_sender(sender: mpsc::Sender<MsgQueueEntry<T>>) -> Self {
        Mailbox { sender }
    }

    /// Send a message to the actor.
    pub fn send<Msg>(self, message: Msg) -> mailbox_futures::MailboxSend<T>
        where
            Msg: Into<MailboxEntry<T>> {

        let msg = MsgQueueEntry::MailboxEntry(message.into());
        mailbox_futures::MailboxSend::new(self,msg)
    }

    /// Send a message to the actor, synchronously, unless there is back pressure.
    ///
    /// If the mailbox is full, the input message will be returned.
    ///
    /// As usual, if the actor is dead, this will swallow that error.
    #[must_use = "send_now will return its input if unable to send now"]
    pub fn send_now<Msg>(&mut self, message: Msg) -> Result<(), MailboxEntry<T>>
        where
            Msg: Into<MailboxEntry<T>> {

        let msg = MsgQueueEntry::MailboxEntry(message.into());
        match self.sender.try_send(msg)
            .err()
            .filter(mpsc::TrySendError::is_full)
            .map(mpsc::TrySendError::into_inner) {

            None => Ok(()),
            Some(rejected) => Err(match rejected {
                MsgQueueEntry::MailboxEntry(entry) => entry,
                _ => unreachable!(),
            }),
        }
    }

    /// Whether the underlying channel is closed.
    ///
    /// If this returns true, the actor is dead. However, this may return false
    /// negatives, where the actor is dead, but the mailbox is still open.
    /// Consequentially, this is difficult to use correctly without introducing
    /// race conditions.
    ///
    /// This method is unlikely to be the correct way to implement code.
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

impl<T: MessageTypeUnion> Clone for Mailbox<T> {
    fn clone(&self) -> Self {
        Mailbox { sender: self.sender.clone() }
    }
}

/// Future types and code for mailboxes.
///
/// Largely boilerplate.
pub mod mailbox_futures {
    use super::*;

    // `Sink` implementation for `Mailbox`
    impl<T: MessageTypeUnion> Sink for Mailbox<T> {
        type SinkItem = MailboxEntry<T>;
        type SinkError = ();

        fn start_send(&mut self, msg: Self::SinkItem) -> StartSend<Self::SinkItem, ()> {
            let msg = MsgQueueEntry::MailboxEntry(msg);
            self.sender.start_send(msg)
                .map(|async_sink|
                    async_sink.map(|rejected| match rejected {
                        MsgQueueEntry::MailboxEntry(entry) => entry,
                        _ => unreachable!(),
                    })
                )
                .map_err(|_| trace!("mailbox Sink::start_send failure"))
        }

        fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
            self.sender.poll_complete()
                .map_err(|_| trace!("mailbox Sink::poll_complete failure"))
        }

        fn close(&mut self) -> Poll<(), Self::SinkError> {
            self.sender.close()
                .map_err(|_| trace!("mailbox Sink::close failure"))
        }
    }

    /// Future for sending into a mailbox.
    pub struct MailboxSend<T>
        where
            T: MessageTypeUnion {
        mailbox: Option<Mailbox<T>>,
        message: Option<MsgQueueEntry<T>>,
    }

    impl<T> MailboxSend<T>
        where
            T: MessageTypeUnion {

        /// Private constructor.
        pub (super) fn new(
            mailbox: Mailbox<T>,
            message: MsgQueueEntry<T>,
        ) -> Self {
            MailboxSend {
                mailbox: Some(mailbox),
                message: Some(message),
            }
        }
    }

    impl<T> Future for MailboxSend<T>
        where
            T: MessageTypeUnion, {

        type Item = Mailbox<T>;
        type Error = ();

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            // mostly delegation to the mailbox channel, except that we swallow error
            // Sender error is caused by actor death

            // take self.mailbox, and un-take it if we yield
            // if self.mailbox is None, we are being invalidly polled after returning Ready
            let mut mailbox = match self.mailbox.take() {
                Some(mailbox) => mailbox,
                None => {
                    warn!("MailboxSend.mailbox is None, was MailboxSend::poll called after completion?");
                    // same behavior as Fuse
                    return Ok(Async::NotReady);
                },
            };

            // delegate to Sender::start_send
            // take self.message, and un-take it if we yield NotReady
            // if self.message is None, Sender::start_send returned Ready in a previous self.poll
            if let Some(message) = self.message.take() {
                match mailbox.sender.start_send(message) {
                    // only if that completes, continue to Sender::poll_complete
                    Ok(AsyncSink::Ready) => (),
                    // elevate NotReady
                    Ok(AsyncSink::NotReady(message)) => {
                        // return state to self
                        self.message = Some(message);
                        self.mailbox = Some(mailbox);

                        return Ok(Async::NotReady);
                    },
                    // swallow error/actor-death
                    Err(_) => return Ok(Async::Ready(mailbox)),
                }
            }

            // delegate to Sender::poll_complete
            match mailbox.sender.poll_complete() {
                Ok(Async::Ready(())) => Ok(Async::Ready(mailbox)),
                Ok(Async::NotReady) => {
                    // return state to self
                    self.mailbox = Some(mailbox);

                    Ok(Async::NotReady)
                },
                // swallow error/actor-death
                Err(_) => Ok(Async::Ready(mailbox)),
            }
        }
    }
}