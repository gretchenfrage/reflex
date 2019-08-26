
//! Actors, simple and fast.

#[macro_use]
extern crate log;
extern crate futures;
extern crate atomic;
extern crate smallvec;
extern crate crossbeam_utils;
extern crate failure;

use crate::msg_union::MessageUnion;

/// Internal concurrency mechanism.
pub (crate) mod internal;

/// Union types for messages which actors can process.
pub mod msg_union;

/// Handles for sending messages to actors.
pub mod mailbox;

// re-export actor guards to the crate root
#[doc(inline)]
pub use crate::internal::{ActorGuardShared, ActorGuardMut};

/// Actor types.
pub trait Actor: Sized + Send + Sync + 'static {
    /// Union type for messages which this actor can process.
    ///
    /// This type is meant to be created with a macro.
    type Message: MessageUnion<Self>;

    /// Type which is produced once upon the explicit termination of this actor.
    ///
    /// TODO: out-mapping impl magic for facilitating decoupling in this situation?
    type End;

    /// The Actor::End type of subordinates to this actor.
    ///
    /// Upon the termination of a subordinate, the supervisor will receive a message
    /// containing the `Actor::End` value of the subordinate, which is produced in
    /// conjunction with the termination of the subordinate.
    ///
    /// In the case of an actor with several subordinate types, macros exist to facilitate
    /// decoupling by creating union types.
    type SubordinateEnd;
}

/// Actor types which can process a particular message type with `&self`.
pub trait ReactShared<Msg>: Actor {
    // TODO: give context? spawn a future? return a future?
    fn process(actor: ActorGuardShared<Self>, message: Msg);
}

/// Actor types which can process a particular message type with `&mut self`.
pub trait ReactMut<Msg>: Actor {
    // TODO: give context? spawn a future? return a future?
    fn process_mut(actor: ActorGuardMut<Self>, message: Msg);
}
