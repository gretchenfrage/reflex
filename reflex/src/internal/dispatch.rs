
use super::*;

use std::hint::unreachable_unchecked;

use futures::prelude::*;
use futures::try_ready;
use futures::task;

/// An actor state is, itself, the dispatch future implementation for the
/// dispatch task.
impl<Act: Actor> Future for ActorState<Act> {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        // if our access count is 0, reset our access status to Available
        //
        // for the case that we were notified by a task which freed up the last
        // actor guard for the current access
        if self.access_status != ActorAccessStatus::Available {
            // atomic-acquire that task's writes to user_state
            //
            // ****
            // | it is important that the atomic acquire occurs before potentially
            // | dropping the future, so that memory barriers are properly instated
            // | for destruction
            // ****
            //
            if self.shared.access_count.load(Ordering::Acquire) == 0 {
                trace!("resetting access_status from {:?} to Available", self.access_status);
                self.access_status = ActorAccessStatus::Available;
            }
        }

        // attempt to grab a message to possibly process
        let msg = {
            try_ready!(self.populate_msg_cell());
            self.curr_msg.take()
        };

        // exit with completion if the mailbox is empty and dropped
        let msg = match msg {
            Some(msg) => msg,
            None => {
                trace!("no more messages, actor terminating");
                return Ok(Async::Ready(()));
            },
        };

        // TODO: this is the point where aforementioned "observing"

        // detect and handle special release modes
        match self.shared.release_mode.swap(
            ReleaseMode::Normal,
            Ordering::Relaxed,
        ) {
            ReleaseMode::Normal => (),
            ReleaseMode::Downgrade => {
                // downgrade access status from Exclusive to Shared
                // unless we already downgraded it to Available
                trace!("downgrading actor guard");
                if self.access_status == ActorAccessStatus::Exclusive {
                    self.access_status = ActorAccessStatus::Shared;
                }
            },
            ReleaseMode::Delete => {
                // terminate actor routine
                // TODO: notify something
                trace!("actor deleted, terminating routine");
                return Ok(Async::Ready(()));
            },
        };

        // yield as not-ready if the message access type isn't compatible with our
        // current access status
        let may_access: bool = {
            let msg_shared: bool = match msg {
                MsgQueueEntry::MailboxEntry(MailboxEntry::Shared(_)) => true,
                MsgQueueEntry::MailboxEntry(MailboxEntry::Mut(_)) => false,
                MsgQueueEntry::SubordinateEnd(_) => false,
            };
            match (msg_shared, self.access_status) {
                (true, ActorAccessStatus::Available) => true,
                (true, ActorAccessStatus::Shared) => true,
                (false, ActorAccessStatus::Available) => true,
                _ => false,
            }
        };
        if !may_access {
            trace!("message access type is not compatible with actor access status, actor blocked");
            self.curr_msg = Some(msg);
            return Ok(Async::NotReady);
        }

        // launch a task to process the message
        // this code manually implements synchronization, so should be entirely considered unsafe

        fn acquire_guard_shared<Act>(actor: &mut ActorState<Act>) -> ActorGuardShared<Act>
        where
            Act: Actor
        {
            trace!("beginning shared actor access");

            let shared_state = Arc::clone(&actor.shared);
            shared_state.access_count.fetch_add(1, Ordering::Relaxed);
            let dispatch_task = task::current();

            actor.access_status = ActorAccessStatus::Shared;
            // create a shared alias into the user_state unsafe cell
            let ptr: *const Act  = unsafe {
                let ptr: &Option<Act> = &*shared_state.user_state.get();
                match ptr {
                    &Some(ref act) => act as *const Act,
                    &None => unreachable_unchecked()
                }
            };
            let actor_guard = ActorGuardShared {
                shared_state,
                dispatch_task,
                ptr,
            };

            actor_guard
        }

        fn acquire_guard_mut<Act>(actor: &mut ActorState<Act>) -> ActorGuardMut<Act>
            where
                Act: Actor
        {
            trace!("beginning exclusive actor access");

            let shared_state = Arc::clone(&actor.shared);
            let previous_access_count = shared_state.access_count.swap(1, Ordering::Relaxed);
            debug_assert_eq!(previous_access_count, 0);
            let dispatch_task = task::current();

            actor.access_status = ActorAccessStatus::Exclusive;
            // create a unique alias into the user_state unsafe cell
            let ptr: *mut Act  = unsafe {
                let ptr: &mut Option<Act> = &mut *shared_state.user_state.get();
                match ptr {
                    &mut Some(ref mut act) => act as *mut Act,
                    &mut None => unreachable_unchecked()
                }
            };
            let actor_guard = ActorGuardMut {
                shared_state,
                dispatch_task,
                ptr,
            };

            actor_guard
        }

        // update synchronization state, and create an actor guard
        // then pass the guard and message to user code
        match msg {
            MsgQueueEntry::MailboxEntry(msg) => match msg {
                MailboxEntry::Shared(msg_vec) => for msg in msg_vec {
                    let guard = acquire_guard_shared(self);
                    Act::handle_msg_shared(guard, msg);
                },
                MailboxEntry::Mut(msg) => {
                    let guard = acquire_guard_mut(self);
                    Act::handle_msg_mut(guard, msg);
                },
            },
            MsgQueueEntry::SubordinateEnd(msg) => {
                let guard = acquire_guard_mut(self);
                Act::handle_subordinate_end(guard, msg);
            },
        };

        // recurse until we terminate or block
        // if the actor processed the message synchronously, this actor may already be released
        self.poll()
    }
}

// helper functions to be called from <ActorState as Future>::poll
impl<Act: Actor> ActorState<Act> {
    /// If `curr_msg` is empty, poll to fill it with a message from `msg_recv`.
    ///
    /// This will only leave `curr_msg` as `None` if `msg_recv` has terminated,
    /// in which case the actor should die.
    #[inline]
    fn populate_msg_cell(&mut self) -> Poll<(), ()> {
        if self.curr_msg.is_none() {
            self.curr_msg = try_ready!(self.msg_recv.poll());
        }
        Ok(Async::Ready(()))
    }
}

