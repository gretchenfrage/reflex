
use crate::Actor;
use crate::msg_union::ActorMessage;

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
///   - `try_send`
///   - `poll_ready`
///   - `is_closed`
pub struct Mailbox<Act: Actor> {
    sender: mpsc::Sender<ActorMessage<Act>>,
}

impl<Act: Actor> Mailbox<Act> {
    /// Crate-internal constructor.
    pub (crate) fn from_sender(sender: mpsc::Sender<ActorMessage<Act>>) -> Self {
        Mailbox {
            sender,
        }
    }

    /// Send a message to the actor.
    pub fn send<Msg>(self, message: Msg) -> mailbox_futures::MailboxSend<Act>
        where
            Msg: Into<ActorMessage<Act>> {

        mailbox_futures::MailboxSend::new(self, message.into())
    }

}

/// Future types and code for mailboxes.
///
/// Largely boilerplate.
pub mod mailbox_futures {
    use super::*;
    use crate::Actor;
    use crate::msg_union::ActorMessage;

    // `Sink` implementation for `Mailbox`
    impl<Act: Actor> Sink for Mailbox<Act> {
        type SinkItem = ActorMessage<Act>;
        type SinkError = ();

        fn start_send(&mut self, msg: Self::SinkItem) -> StartSend<Self::SinkItem, ()> {
            self.sender.start_send(msg)
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
    pub struct MailboxSend<Act>
        where
            Act: Actor {
        mailbox: Option<Mailbox<Act>>,
        message: Option<ActorMessage<Act>>,
    }

    impl<Act> MailboxSend<Act>
        where Act: Actor {

        /// Private constructor.
        pub (super) fn new(
            mailbox: Mailbox<Act>,
            message: ActorMessage<Act>,
        ) -> Self {
            MailboxSend {
                mailbox: Some(mailbox),
                message: Some(message),
            }
        }
    }

    impl<Act> Future for MailboxSend<Act>
        where
            Act: Actor, {

        type Item = Mailbox<Act>;
        type Error = ();

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            // mostly delegation to the mailbox channel, except that we swallow error
            // Sender error is caused by actor death

            // take self.mailbox, and return it if we yield
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
            // take self.message, and return it if we yield NotReady
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

