
//! Actors, simple and fast.

#[macro_use]
extern crate log;
extern crate futures;
extern crate atomic;
extern crate smallvec;
extern crate failure;

use crate::msg_union::{Message, MessageTypeUnion};

/// Internal concurrency mechanism.
pub (crate) mod internal;

/// Internal utilities which could be factored out.
pub (crate) mod util;

/// Union types for messages which actors can process.
pub mod msg_union;

/// Handles for sending messages to actors.
pub mod mailbox;

/// Actors supervising other actors.
pub mod supervise;

// re-export actor guards to the crate root
#[doc(inline)]
pub use crate::internal::{ActorGuardShared, ActorGuardMut};

/// Actor types.
///
/// This trait is meant to be implemented with macros.
pub trait Actor: Sized + Send + Sync + 'static {
    /// Message types which this actor can process.
    ///
    /// This type is meant to be implemented with macros.
    type Message: MessageTypeUnion;

    /// Type which signals the intentional termination of this actor.
    ///
    /// Terminating an actor through an `ActorGuardMut` requires the provision of an
    /// instance of its end value.
    type End: Message;

    /// The Actor::End type of subordinates to this actor.
    ///
    /// Upon the termination of a subordinate, the supervisor will receive a message
    /// containing the `Actor::End` value of the subordinate, which is produced in
    /// conjunction with the termination of the subordinate.
    ///
    /// In the case of an actor with several subordinate types, macros exist to
    /// facilitate decoupling by creating union types.
    type SubordinateEnd: Message;

    fn handle_msg_shared(
        actor: ActorGuardShared<Self>,
        msg: <Self::Message as MessageTypeUnion>::SharedUnion,
    );

    fn handle_msg_mut(
        actor: ActorGuardMut<Self>,
        msg: <Self::Message as MessageTypeUnion>::MutUnion,
    );

    fn handle_subordinate_end(
        actor: ActorGuardMut<Self>,
        msg: Self::SubordinateEnd,
    );
}

/// Actor types which can process a particular message type with `&self`.
pub trait ReactShared<Msg>: Actor + Sized {
    // TODO: give context? spawn a future? return a future?
    fn process(actor: ActorGuardShared<Self>, message: Msg);
}

/// Actor types which can process a particular message type with `&mut self`.
pub trait ReactMut<Msg>: Actor + Sized {
    // TODO: give context? spawn a future? return a future?
    fn process_mut(actor: ActorGuardMut<Self>, message: Msg);
}
