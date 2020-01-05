
use self::{
    generic::Mailbox,
    ownership::{Supervisor, Weak},
};
use crate::msg_union::MessageTypeUnion;

/// Handling of actor ownership semantics.
pub mod ownership;

/// Code that is generic over actor-ownership semantics.
///
/// More usable type aliases exist in this parent module.
pub mod generic;

/// A handle for sending messages to an actor, which keeps the actor alive.
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
pub type MailboxOwned<Msg> = Mailbox<Msg, Supervisor>;

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
pub type MailboxWeak<Msg> = Mailbox<Msg, Weak>;

impl<Msg: MessageTypeUnion> MailboxOwned<Msg> {
    /// Downgrade into a weak mailbox, which will not keep the actor alive.
    pub fn downgrade(self) -> MailboxWeak<Msg> {
        Mailbox {
            sender: self.sender,
            ownership: Weak,
        }
    }
}