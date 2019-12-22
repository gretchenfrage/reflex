
use crate::util::drop_signal::DropSignalArc;

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

impl Ownership for Supervisor {}

/// Weak ownership semantics.
///
/// This mailbox will not keep the actor alive. This is probably used to create direct
/// links between cousin actors.
#[derive(Copy, Clone, Debug)]
pub struct Weak;

impl Ownership for Weak {}