
use super::*;
use crate::msg_union::{MessageUnionMut, MessageUnionShared};

use futures::prelude::*;
use futures::try_ready;
use futures::task;

/// Set up the internal concurrency mechanism for an actor.
///
/// Returns:
/// 1. the actor state, which, itself, is a the dispatch task future
/// 2. a message sender handle
pub fn create_actor<Act: Actor>(user_state: Act) -> (
    ActorState<Act>,
    mpsc::Sender<ActorMessage<Act>>,
) {
    // create the message channel
    let (msg_send, msg_recv) = mpsc::channel(1000);

    // create the shared state
    let state_shared = ActorStateShared {
        user_state: CachePadded::new(UnsafeCell::new(user_state)),
        access_count: CachePadded::new(Atomic::new(0)),
    };
    let state_shared = Arc::new(state_shared);

    // create the owned state
    let state = ActorState {
        shared: state_shared,
        access_status: ActorAccessStatus::Available,
        msg_recv,
        curr_msg: None,
    };

    // return
    (state, msg_send)
}

/// An actor state is, itself, the dispatch future implementation for the
/// dispatch task.
impl<Act: Actor> Future for ActorState<Act> {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        // attempt to grab a message to possibly process
        let msg = {
            try_ready!(self.populate_msg_cell());
            self.curr_msg.take()
        };

        // exit with completion if there are no more messages to process
        let msg = match msg {
            Some(msg) => msg,
            None => {
                // TODO: notify something
                trace!("no more messages, actor terminating");
                return Ok(Async::Ready(()));
            },
        };

        // if our access count is 0, reset our access status to Available
        // for the case that we were notified by a task which freed up the last actor guard
        // for the current access
        if self.access_status != ActorAccessStatus::Available {
            // atomic-acquire that task's writes to user_state
            if self.shared.access_count.load(Ordering::Acquire) == 0 {
                trace!("resetting access_status from {:?} to Available", self.access_status);
                self.access_status = ActorAccessStatus::Available;
            }
        }

        // yield as not-ready if the message access type isn't compatible with our
        // current access status
        let may_access = match (&msg, self.access_status) {
            (&ActorMessage::Shared(_), ActorAccessStatus::Available) => true,
            (&ActorMessage::Shared(_), ActorAccessStatus::Shared) => true,
            (&ActorMessage::Mut(_), ActorAccessStatus::Available) => true,
            _ => false,
        };
        if !may_access {
            trace!("message access type is not compatible with actor access status, actor blocked");
            self.curr_msg = Some(msg);
            return Ok(Async::NotReady);
        }

        // launch a task to process the message
        // this code manually implements synchronization, so should be entirely considered unsafe
        unsafe {
            // update synchronization state, and create an actor guard
            let shared_state = self.shared.clone();
            let previous_access_count = shared_state.access_count.fetch_add(1, Ordering::Relaxed);
            let dispatch_task = task::current();

            // the remainder is specialized for access type
            match msg {
                ActorMessage::Shared(msg) => {
                    trace!("beginning shared actor access");

                    self.access_status = ActorAccessStatus::Shared;
                    // create a shared alias into the user_state unsafe cell
                    let ptr: *const Act = shared_state.user_state.get();
                    let actor_guard = ActorGuardShared {
                        shared_state,
                        dispatch_task,
                        ptr,
                    };

                    // delegate to the message union
                    // unlocking the actor is performed in the actor guard destructor
                    MessageUnionShared::process(msg, actor_guard);
                },
                ActorMessage::Mut(msg) => {
                    trace!("beginning exclusive actor access");

                    self.access_status = ActorAccessStatus::Exclusive;
                    debug_assert_eq!(previous_access_count, 0);
                    // create a unique alias into the user_state unsafe cell
                    let ptr: *mut Act = shared_state.user_state.get();
                    let actor_guard = ActorGuardMut {
                        shared_state,
                        dispatch_task,
                        ptr,
                    };

                    // delegate to the message union
                    // unlocking the actor is performed in the actor guard destructor
                    MessageUnionMut::process(msg, actor_guard);
                }
            }
        }

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