
#[macro_use]
extern crate log;
extern crate futures;
extern crate atomic;
extern crate smallvec;
extern crate crossbeam_utils;

/// Internal concurrency mechanism.
mod internal;

/// Union types for messages which actors can process.
pub mod msg_union;

use msg_union::MessageUnion;

/// Actor types.
pub trait Actor: Sized + Send + Sync + 'static {
    /// Union type for messages which this actor can process.
    type Message: MessageUnion<Self>;
}

/// Actor types which can process a particular message type with `&self`.
pub trait ReactShared<Msg>: Actor {
    // TODO: fn process with actor guard
}

/// Actor types which can process a particular message type with `&mut self`.
pub trait ReactMut<Msg>: Actor {
    // TODO: fn process with actor guard
}

