
use crate::Actor;
use super::ActorStateShared;
use crate::manage::SubordinateActor;
use crate::mailbox::MailboxOwned;
use super::create::{
    create_actor,
    create_actor_using_mailbox,
};

/// Create a subordinate actor, given the manager actor's shared state.
pub fn create_subordinate<Act1, Act2>(
    supervisor: &ActorStateShared<Act1>,
    subordinate: Act2,
) -> (SubordinateActor<Act2>, MailboxOwned<Act2::Message>)
where
    Act1: Actor,
    Act2: Actor<End = Act1::SubordinateEnd>,
{
    let (
        actor,
        msg_sender,
        drop_signal_send,
    ) = create_actor(
        subordinate,
        supervisor.subord_end_signal_send.clone(),
    );

    (
        SubordinateActor::new(actor),
        MailboxOwned::new_owned(
            msg_sender,
            drop_signal_send.arc(),
        ),
    )
}