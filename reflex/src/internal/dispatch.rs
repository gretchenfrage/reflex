
use super::*;

use futures::prelude::*;
use futures::try_ready;

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
            None => return Ok(Async::Ready(())),
        };

        unimplemented!()
    }
}

// helper functions to be called from <ActorState as Future>::poll
impl<Act: Actor> ActorState<Act> {
    /// If `curr_msg` is empty, poll to fill it with a message from `msg_recv`.
    ///
    /// This will only leave `curr_msg` as `None` if `msg_recv` has terminated,
    /// in which case the actor should die.
    fn populate_msg_cell(&mut self) -> Poll<(), ()> {
        if self.curr_msg.is_none() {
            self.curr_msg = try_ready!(self.msg_recv.poll());
        }
        Ok(Async::Ready(()))
    }
}