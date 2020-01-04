
use crate::{
    Actor,
    mailbox::{
        MailboxOwned,
        MailboxWeak,
    },
    internal::{
        create::{
            create_actor,
            create_actor_using_mailbox,
        },
        ActorState,
    },
};

use futures::{
    {Future, Stream, Poll, Async},
    sync::mpsc,
    future::Fuse,
};

#[cfg(feature = "failure-interop")]
use failure::Fail;
#[cfg(feature = "failure-interop")]
use std::fmt::{self, Display, Formatter};

/// Handle to an owned actor with no manager.
///
/// In other words, this handle is the manager. A `RootActor` is a future,
/// and will resolve to the actor's end message.
///
/// This handle also contains and exposes the actor's mailbox.
pub struct RootActor<Act>
where
    Act: Actor,
    Act::End: IntoResult,
{
    actor: Fuse<ActorState<Act>>,
    mailbox: MailboxOwned<Act::Message>,
    end_signal_recv: mpsc::UnboundedReceiver<Act::End>,
}

impl<Act> RootActor<Act>
where
    Act: Actor,
    Act::End: IntoResult,
    <Act::End as IntoResult>::Error: From<AbnormalClose>,
{
    /// Create a new root actor. It still must be spawned onto an executor.
    pub fn new(state: Act) -> Self {
        let (
            end_signal_send,
            end_signal_recv,
        ) = mpsc::unbounded();

        let (
            actor,
            msg_sender,
            drop_signal_send,
        ) = create_actor(state, end_signal_send);

        let mailbox = MailboxOwned::new_owned(msg_sender, drop_signal_send.arc());

        RootActor {
            actor: actor.fuse(),
            mailbox,
            end_signal_recv,
        }
    }

    pub fn mailbox(&self) -> &MailboxOwned<Act::Message> {
        &self.mailbox
    }

    pub fn mailbox_mut(&mut self) -> &MailboxOwned<Act::Message> {
        &mut self.mailbox
    }

    /// Clone and downgrade the mailbox.
    pub fn mailbox_handle(&self) -> MailboxWeak<Act::Message> {
        self.mailbox.clone().downgrade()
    }
}

impl<Act> Future for RootActor<Act>
where
    Act: Actor,
    Act::End: IntoResult,
    <Act::End as IntoResult>::Error: From<AbnormalClose>,
{
    type Item = <Act::End as IntoResult>::Item;
    type Error = <Act::End as IntoResult>::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let _ = self.actor.poll();

        match self.end_signal_recv.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(Some(end))) => (
                end.into_result().map(Async::Ready)
            ),
            Ok(Async::Ready(_)) => (
                Err(Self::Error::from(AbnormalClose))
            ),
            Err(_) => (
                Err(Self::Error::from(AbnormalClose))
            ),
        }
    }
}

/// A future that drives a subordinate actor.
///
/// This is returned from using an actor guard to spawn a subordinate.
/// This must be spawned onto an executor, and that's the only thing
/// you can do with this.
pub struct SubordinateActor<Act: Actor> {
    // this is just an opaque wrapper around crate::internal::ActorState
    actor: ActorState<Act>,
}

impl<Act: Actor> SubordinateActor<Act> {
    /// Crate-internal constructor.
    pub (crate) fn new(actor: ActorState<Act>) -> Self {
        SubordinateActor { actor }
    }
}

impl<Act: Actor> Future for SubordinateActor<Act> {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.actor.poll()
    }
}

// error handling

/// Error which denotes that an actor closed without properly producing an `End`
/// message.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AbnormalClose;

#[cfg(feature = "failure-interop")]
impl Fail for AbnormalClose {}

#[cfg(feature = "failure-interop")]
impl Display for AbnormalClose {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("future abnormally closed")
    }
}

/// Type that can synchronously convert into a result.
pub trait IntoResult {
    type Item;
    type Error;

    fn into_result(self) -> Result<Self::Item, Self::Error>
    where
        Self: Sized;
}

impl<I, E> IntoResult for Result<I, E> {
    type Item = I;
    type Error = E;

    fn into_result(self) -> Result<Self::Item, Self::Error>
    where
        Self: Sized,
    {
        self
    }
}
