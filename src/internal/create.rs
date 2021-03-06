
use super::*;
use super::queue::MsgQueue;
use crate::msg_union::ActorMailboxEntry;
use crate::util::drop_signal::{DropSignalSend, drop_signal_channel};

/// Set up the internal concurrency mechanism for an actor.
///
/// Returns:
/// 1. the actor state, which, itself, is the dispatch task future
/// 2. a message sender handle
/// 3. the drop signal sender which signals that the actor is orphaned
pub fn create_actor<Act: Actor>(
    user_state: Act,
    end_signal_send: mpsc::UnboundedSender<Act::End>
) -> (
    ActorState<Act>,
    mpsc::Sender<ActorMailboxEntry<Act>>,
    DropSignalSend,
) {
    // create the message channels
    let (
        mailbox_send,
        mailbox_recv
    ) = mpsc::channel(1000);
    let (
        sub_end_send,
        sub_end_recv
    ) = mpsc::unbounded();
    let (
        kil_sig_send,
        kil_sig_recv,
    ) = drop_signal_channel();

    let msg_recv = MsgQueue::new(
        kil_sig_recv,
        mailbox_recv,
        sub_end_recv,
    );

    // create the actor state
    let state = create_actor_using_mailbox(
        user_state,
        msg_recv,
        end_signal_send,
        sub_end_send,
    );
    
    // return
    (state, mailbox_send, kil_sig_send)
}

/// Set up the internal concurrency mechanism for an actor, except its
/// mailbox, which already exists.
///
/// The mailbox channel receiver is given as a parameter. This returns
/// the actor state, which, itself, is the dispatch task future.
pub fn create_actor_using_mailbox<Act: Actor>(
    user_state: Act,
    msg_recv: MsgQueue<Act>,
    end_signal_send: mpsc::UnboundedSender<Act::End>,
    subord_end_signal_send: mpsc::UnboundedSender<<Act as Actor>::SubordinateEnd>,
) -> ActorState<Act> {
    // create the shared state
    let state_shared = ActorStateShared {
        user_state: UnsafeCell::new(Some(user_state)),
        access_count: Atomic::new(0),
        release_mode: Atomic::new(ReleaseMode::Normal),
        end_signal_send,
        subord_end_signal_send,
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