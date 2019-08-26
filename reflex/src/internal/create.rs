
use super::*;

/// Set up the internal concurrency mechanism for an actor.
///
/// Returns:
/// 1. the actor state, which, itself, is the dispatch task future
/// 2. a message sender handle
pub fn create_actor<Act: Actor>(user_state: Act) -> (
    ActorState<Act>,
    mpsc::Sender<ActorMessage<Act>>,
) {
    // create the message channel
    let (msg_send, msg_recv) = mpsc::channel(1000);

    // create the actor state
    let state = create_actor_with_mailbox(user_state, msg_recv);
    
    // return
    (state, msg_send)
}

/// Set up the internal concurrency mechanism for an actor, except its
/// mailbox, which already exists.
///
/// The mailbox channel receiver is given as a parameter. This returns
/// the actor state, which, itself, is the dispatch task future.
pub fn create_actor_with_mailbox<Act: Actor>(
    user_state: Act,
    msg_recv: mpsc::Receiver<ActorMessage<Act>>
) -> ActorState<Act> {
    // create the shared state
    let state_shared = ActorStateShared {
        user_state: CachePadded::new(UnsafeCell::new(user_state)),
        access_count: CachePadded::new(Atomic::new(0)),
    };
    let state_shared = Arc::new(state_shared);

    // create the owned state
    ActorState {
        shared: state_shared,
        access_status: ActorAccessStatus::Available,
        msg_recv,
        curr_msg: None,
    }
}

// TODO: async creation