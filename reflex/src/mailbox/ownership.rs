
use super::MailboxOwned;
use super::generic::Mailbox;
use crate::util::drop_signal::DropSignalArc;
use crate::msg_union::{MessageTypeUnion, MailboxEntry};


use futures::sync::mpsc;

/// Mechanism for mailbox actor-ownership semantics.
pub trait Ownership: Clone + Send + Sync + 'static {}

/// Supervisor ownership semantics.
///
/// A mailbox with this ownership type will keep the actor alive until the mailbox
/// is dropped.
#[derive(Clone)]
pub struct Supervisor {
    drop_signal: DropSignalArc
}

impl Supervisor {
    /// Crate-internal constructor.
    pub (crate) fn new(drop_signal: DropSignalArc) -> Self {
        Supervisor { drop_signal }
    }
}

impl Ownership for Supervisor {}

/// Weak ownership semantics.
///
/// This mailbox will not keep the actor alive. This is probably used to create direct
/// links between cousin actors.
#[derive(Copy, Clone, Debug)]
pub struct Weak;

impl Ownership for Weak {}



impl<T: MessageTypeUnion> MailboxOwned<T> {
    /// Crate-internal constructor.
    pub (crate) fn new_owned(
        msg_send: mpsc::Sender<MailboxEntry<T>>,
        drop_signal: DropSignalArc,
    ) -> Self {
        Mailbox::new(msg_send, Supervisor::new(drop_signal))
    }
}