
use crate::Actor;
use super::ActorStateShared;
use crate::manage::{SubordinateActor, ActorSocket};
use crate::mailbox::MailboxOwned;
use super::create::create_actor;
use super::queue::MsgQueue;
use futures::sync::mpsc;
use crate::util::drop_signal::drop_signal_channel;


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

/// Create a subordinate actor socket, given the manager actor's
/// shared state.
pub fn create_subordinate_socket<Act1, Act2>(
    supervisor: &ActorStateShared<Act1>,
) -> (ActorSocket<Act2>, MailboxOwned<Act2::Message>)
where
    Act1: Actor,
    Act2: Actor<End = Act1::SubordinateEnd>,
{
    // create the message channels
    // TODO: this is a copy/paste of code in `super::create`
    //       and these fields are in severe need of packaging
    //       into a struct
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

    (
        ActorSocket::new(
            msg_recv,
            supervisor.subord_end_signal_send.clone(),
            sub_end_send,
        ),
        MailboxOwned::new_owned(
            mailbox_send,
            kil_sig_send.arc(),
        ),
    )
}