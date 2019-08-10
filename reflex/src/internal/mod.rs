
use crate::Actor;

use std::sync::Arc;
use std::cell::UnsafeCell;

use atomic::{Atomic, Ordering};
use crossbeam_utils::CachePadded;

/// Reflex's state for an actor which is owned by the actor's dispatch routine.
pub struct ActorState<Act: Actor> {
    // handle to the shared state
    shared: Arc<ActorStateShared<Act>>,

    // exclusively owned state
    access_status: ActorAccessStatus
}

/// Reflex's state for an actor which is reference counted.
pub struct ActorStateShared<Act: Actor> {
    // we will cache-pad these two pieces of state

    // the user's actor struct, which we manually synchronize
    user_state: CachePadded<UnsafeCell<Act>>,
    // the current number of guards accessing (mutably or immutably) the user state
    access_count: CachePadded<Atomic<u32>>,
}

/// The way in which an actor is currently being accessed, equivalent to the state of a
/// read/write lock.
pub enum ActorAccessStatus {
    /// The actor is not being accessed.
    Available,
    /// The actor is being immutably accessed, and is available for further concurrent
    /// immutable access.
    Shared,
    /// The actor is experiencing mutually exclusive, mutable access.
    Exclusive,
}

