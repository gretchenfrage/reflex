
use crate::Actor;

use std::sync::Arc;
use std::cell::UnsafeCell;

use atomic::{Atomic, Ordering};
use crossbeam_utils::CachePadded;
use futures::task::Task;

/// Implementation of dereferencing actor guards.
mod actor_guard_deref;

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

/// Synchronization guard for shared (immutable) access to an actor.
///
/// This type is notably `'static`, and clone-shareable.
pub struct ActorGuardShared<Act: Actor> {
    // handle to the shared state
    shared_state: Arc<ActorStateShared<Act>>,
    // handle to the actor's dispatch task, to wake it up when it unblocks the task
    dispatch_task: Task,
    // cache a pointer, for better aliasing
    ptr: *const Act,
}

/// Synchronization guard for exclusive (mutable) access to an actor.
///
/// This type is notably `'static`.
pub struct ActorGuardMut<Act: Actor> {
    // handle to the shared state
    shared_state: Arc<ActorStateShared<Act>>,
    // handle to the actor's dispatch task, to wake it up when it unblocks the task
    dispatch_task: Task,
    // cache a pointer, for better aliasing
    ptr: *mut Act,
}

// TODO: clone shared actor guard
// TODO: drop actor guard