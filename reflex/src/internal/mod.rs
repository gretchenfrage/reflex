
/// Internal tests.
#[cfg(test)]
mod test;

use self::queue::{MsgQueue, MsgQueueEntry};
use crate::Actor;
use crate::msg_union::ActorMailboxEntry;

use std::sync::Arc;
use std::cell::UnsafeCell;

use atomic::{Atomic, Ordering};
use futures::sync::mpsc;
use futures::task::Task;

/// Actor dispatch task.
pub mod dispatch;

/// Actor guard drop implementations.
pub mod actor_guard_impl;

/// Internal creation of actors.
pub mod create;

/// Abstraction over message queues.
pub mod queue;

/// Reflex's state for an actor which is owned by the actor's dispatch routine.
pub struct ActorState<Act: Actor> {
    // handle to the shared state
    shared: Arc<ActorStateShared<Act>>,

    access_status: ActorAccessStatus,

    // the message queue, and the slot for pushing a message back in
    msg_recv: MsgQueue<Act>,
    curr_msg: Option<MsgQueueEntry<Act>>,
}


/// Reflex's state for an actor which is reference counted.
pub struct ActorStateShared<Act: Actor> {
    // the user's actor struct, which we manually synchronize
    // additionally, we make unsafe assumptions on when this is the Some variant
    user_state: UnsafeCell<Option<Act>>,

    // the current number of guards accessing (mutably or immutably) the user state
    access_count: Atomic<u32>,
    // atomic memory for an actor guard to tell the internal actor procedure that
    // the guard is releasing in some relevant non-default way
    release_mode: Atomic<ReleaseMode>,

    // channel to notify manager actor of explicit termination
    end_signal_send: mpsc::UnboundedSender<<Act as Actor>::End>,
}

/// The way in which an actor is currently being accessed, equivalent to the state of a
/// read/write lock.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ActorAccessStatus {
    /// The actor is not being accessed.
    Available,
    /// The actor is being immutably accessed, and is available for further concurrent
    /// immutable access.
    Shared,
    /// The actor is experiencing mutually exclusive, mutable access.
    Exclusive,
}

/// A possible indicator of an actor guard releasing in some non-default way.
///
/// This warrants special handling by the internal actor state. This value
/// is conveyed to the internal actor state through atomic operations.
///
/// TODO: make a note here on the non-obvious semantics of what it means for the
///       actor routine to "observe" this
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ReleaseMode {
    /// There is nothing notable about the release mode.
    ///
    /// If the actor routine observes:
    /// ```no_run,noplaypen
    /// release_mode = Normal
    /// access_count = 0
    /// ```
    /// Then the actor routine should assume that the actor is in an
    /// *unlocked* state.
    Normal,
    /// An `ActorGuardMut` is downgrading itself to an `ActorGuardShared`, allowing
    /// other threads to concurrently read.
    ///
    /// If the actor routine observes:
    /// ```no_run,noplaypen
    /// release_mode = Downgrade
    /// access_count = 1
    /// ```
    /// Then the actor routine should assume that the actor is in an
    /// *unlocked* state.
    ///
    /// If the actor routine observes:
    /// ```no_run,noplaypen
    /// release_mode = Downgrade
    /// access_count = 0
    /// ```
    /// This suggests that a mutable guard downgraded, then released, before the
    /// actor routine was polled in time to observe its downgraded state. The actor
    /// should treat this like it would the *unlocked* state in general (but make sure
    /// to reset the release mode to `Normal`).
    ///
    /// It is **invalid** for the actor routine to observe:
    /// ```no_run,noplaypen
    /// release_mode = Downgrade
    /// access_count > 1
    /// ```
    Downgrade,
    /// An `ActorGuardMut` is deleting the actor.
    ///
    /// If the actor routine observes:
    /// ```no_run,noplaypen
    /// release_mode = Delete
    /// access_count = 0
    /// ```
    /// Then the actor routine should assume that the `user_state` has been set
    /// to `None`, and that the actor should terminate.
    ///
    /// In this scenario, the thread which triggered the deletion took ownership
    /// of the user state, and is therefore responsible for dropping it.
    ///
    /// Furthermore, this is the only situation in which it is allowed for the actor
    /// routine not to observe `user_state` in the `Some` variant.
    ///
    /// It is **invalid** for the actor routine to observe:
    /// ```no_run,noplaypen
    /// release_mode = Delete
    /// access_count != 0
    /// ```
    Delete,
}

/// Synchronization guard for shared (immutable) access to an actor.
///
/// This type is notably `'static`, and clone-shareable.
#[repr(C)]
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
#[repr(C)]
pub struct ActorGuardMut<Act: Actor> {
    // handle to the shared state
    shared_state: Arc<ActorStateShared<Act>>,
    // handle to the actor's dispatch task, to wake it up when it unblocks the task
    dispatch_task: Task,
    // cache a pointer, for better aliasing
    ptr: *mut Act,
}

unsafe impl<Act: Send + Actor> Send for ActorGuardShared<Act> {}
unsafe impl<Act: Sync + Actor> Sync for ActorGuardShared<Act> {}
unsafe impl<Act: Send + Actor> Send for ActorGuardMut<Act> {}
unsafe impl<Act: Sync + Actor> Sync for ActorGuardMut<Act> {}


unsafe impl<Act> Send for ActorState<Act>
    where
        Act: Actor + Send + Sync,
        ActorMailboxEntry<Act>: Send {}

unsafe impl<Act> Sync for ActorState<Act>
    where
        Act: Actor + Send + Sync,
        ActorMailboxEntry<Act>: Send {}