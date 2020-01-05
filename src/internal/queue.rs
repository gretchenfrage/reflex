
use crate::{
    Actor,
    msg_union::MailboxEntry,
    util::drop_signal::DropSignalRecv,
};

use futures::{
    Stream, Future, Poll, Async,
    sync::mpsc,
    stream::Fuse,
};

/// Abstraction over actor message queues.
///
/// Not round-robin; prioritizes queues over other queues, in a deliberate order.
///
/// May be polled after completion without concern.
///
/// Instead of actually emitting an element upon receiving the drop signal, the queue
/// simply terminates. This may change in the future, to facilitate drop recovery.
pub struct MsgQueue<Act: Actor> {
    kil_sig_recv: DropSignalRecv,
    sub_end_recv: Fuse<mpsc::UnboundedReceiver<
        <Act as Actor>::SubordinateEnd
    >>,
    mailbox_recv: Fuse<mpsc::Receiver<
        MailboxEntry<<Act as Actor>::Message>
    >>,
}

/// Element type of `MsgQueue`.
pub enum MsgQueueEntry<Act: Actor> {
    MailboxEntry(MailboxEntry<<Act as Actor>::Message>),
    SubordinateEnd(<Act as Actor>::SubordinateEnd),
}

impl<Act: Actor> MsgQueue<Act> {
    pub fn new(
        kil_sig_recv: DropSignalRecv,
        mailbox_recv: mpsc::Receiver<MailboxEntry<<Act as Actor>::Message>>,
        sub_end_recv: mpsc::UnboundedReceiver<<Act as Actor>::SubordinateEnd>,
    ) -> Self {
        MsgQueue {
            kil_sig_recv,
            mailbox_recv: mailbox_recv.fuse(),
            sub_end_recv: sub_end_recv.fuse(),
        }
    }
}

impl<Act: Actor> Stream for MsgQueue<Act> {
    type Item = MsgQueueEntry<Act>;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut blocked = false;

        match self.kil_sig_recv.poll() {
            Ok(Async::Ready(())) => return Ok(Async::Ready(None)),
            Ok(Async::NotReady) => (),

            Err(never) => match never {},
        };

        fn async_flatten<'a, A, B, F: FnOnce(A) -> B + 'a>(
            map: F,
            blocked: &'a mut bool,
        ) -> impl FnOnce(Async<Option<A>>) -> Option<B> + 'a {
            move |asnc| match asnc {
                Async::Ready(option) => option.map(map),
                Async::NotReady => {
                    *blocked = true;
                    None
                },
            }
        }

        self.sub_end_recv.poll()
            .map(async_flatten(MsgQueueEntry::SubordinateEnd, &mut blocked))
            .transpose()
            .or_else(|| self.mailbox_recv
                .poll()
                .map(async_flatten(MsgQueueEntry::MailboxEntry, &mut blocked))
                .transpose()
            )
            .transpose()
            .map(|option| match option {
                None if blocked => Async::NotReady,
                None => Async::Ready(None),
                Some(elem) => Async::Ready(Some(elem))
            })
    }
}