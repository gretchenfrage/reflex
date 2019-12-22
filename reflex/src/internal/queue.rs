
use crate::{
    Actor,
    msg_union::MailboxEntry,
};

use futures::{
    Stream, Poll, Async,
    sync::mpsc,
    stream::Fuse,
};

/// Abstraction over actor message queues.
///
/// Contains the unbounded subordinate-end queue, and the bounded mailbox-recv queue.
/// The subordinate-end queue is drained with higher priority.
pub struct MsgQueue<Act: Actor> {
    mailbox_recv: Fuse<mpsc::Receiver<
        MailboxEntry<<Act as Actor>::Message>
    >>,
    subordinate_end_recv: Fuse<mpsc::UnboundedReceiver<
        <Act as Actor>::SubordinateEnd
    >>,
}

/// Element type of `MsgQueue`.
pub enum MsgQueueEntry<Act: Actor> {
    MailboxEntry(MailboxEntry<<Act as Actor>::Message>),
    SubordinateEnd(<Act as Actor>::SubordinateEnd),
}

impl<Act: Actor> Stream for MsgQueue<Act> {
    type Item = MsgQueueEntry<Act>;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let a = &mut self.subordinate_end_recv;
        let b = &mut self.mailbox_recv;

        let mut blocked = false;

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

        a.poll()
            .map(async_flatten(MsgQueueEntry::SubordinateEnd, &mut blocked))
            .transpose()
            .or_else(|| b
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