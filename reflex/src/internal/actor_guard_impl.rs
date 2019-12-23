
use crate::Actor;
use super::{ActorGuardShared, ActorGuardMut, ReleaseMode};

use std::ops::{Deref, DerefMut};
use std::mem;
use std::hint::unreachable_unchecked;

use atomic::Ordering;

impl<Act: Actor> ActorGuardMut<Act> {
    /// Downgrade from mutable to shared.
    ///
    /// This unblocks other concurrent actor accesses.
    pub fn downgrade(guard: Self) -> ActorGuardShared<Act> {
        guard.shared_state.release_mode.store(
            ReleaseMode::Downgrade,
            Ordering::Relaxed,
        );

        // notify task so that more messages can be processed
        guard.dispatch_task.notify();

        // it's very important that guards have the same internal representation
        // because, to avoid dropping annoyances, we'll just transmute them
        // that's why they have repr(C)
        unsafe {
            mem::transmute::<
                ActorGuardMut<Act>,
                ActorGuardShared<Act>,
            >(guard)
        }
    }

    /// Delete this actor, and extract the inner state.
    pub fn delete(guard: Self, end: Act::End) -> Act {
        // send the end message to this actor's manager
        let res = guard.shared_state.end_signal_send.unbounded_send(end);
        if res.is_err() {
            trace!("actor was explicitly terminated, but parent is already dead");
        }

        // extract user state
        // be careful, because this invalidates our internal ptr
        let user_state: Act = unsafe {
            let cell: *mut Option<Act> = guard.shared_state.user_state.get();
            match (&mut *cell).take() {
                Some(user_state) => user_state,
                None => unreachable_unchecked(),
            }
        };

        // if this release mode isn't set, the old memory for the user state
        // may be invalidly accessed
        guard.shared_state.release_mode.store(
            ReleaseMode::Delete,
            Ordering::Relaxed,
        );

        // the dropping of self will handle task notification
        mem::drop(guard);

        user_state
    }
}

// == drop impls ==

impl<Act: Actor> Drop for ActorGuardShared<Act> {
    fn drop(&mut self) {
        // decrement the access_count, and if we've lowered it to 0, notify the task
        let previous_access_count = self.shared_state.access_count.fetch_sub(1, Ordering::Relaxed);
        if previous_access_count == 1 {
            trace!("access count lowered to 0, notifying dispatch task");
            self.dispatch_task.notify();
        }
    }
}

impl<Act: Actor> Drop for ActorGuardMut<Act> {
    fn drop(&mut self) {
        // decrement the access_count
        // since we have exclusive access, this should decrement it to 0
        // atomic-release our user_state writes to the dispatch task
        let previous_access_count = self.shared_state.access_count.swap(0, Ordering::Release);
        debug_assert_eq!(previous_access_count, 1);

        trace!("exclusive actor guard released, notifying dispatch task");
        self.dispatch_task.notify();
    }
}

// implementation for cloning the shared actor guard
impl<Act: Actor> Clone for ActorGuardShared<Act> {
    fn clone(&self) -> Self {
        // increment the access count, then the rest is straight-forward cloning
        self.shared_state.access_count.fetch_add(1, Ordering::Relaxed);

        ActorGuardShared {
            shared_state: self.shared_state.clone(),
            dispatch_task: self.dispatch_task.clone(),
            ptr: self.ptr,
        }
    }
}

// == deref impls ==

impl<Act: Actor> Deref for ActorGuardShared<Act> {
    type Target = Act;

    fn deref(&self) -> &Act {
        unsafe {
            &*self.ptr
        }
    }
}

impl<Act: Actor> Deref for ActorGuardMut<Act> {
    type Target = Act;

    fn deref(&self) -> &Act {
        unsafe {
            &*self.ptr
        }
    }
}

impl<Act: Actor> DerefMut for ActorGuardMut<Act> {
    fn deref_mut(&mut self) -> &mut Act {
        unsafe {
            &mut *self.ptr
        }
    }
}
