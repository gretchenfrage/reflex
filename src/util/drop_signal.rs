
use std::{
    sync::Arc,
    convert::Infallible,
};

use futures::{
    Future,
    Poll,
    Async,
    sync::oneshot,
};

/// Paired with a `DropSignalRecv`, sends the signal when this is dropped.
///
/// A user can also send the signal early, without dropping.
pub struct DropSignalSend {
    send: Option<oneshot::Sender<()>>
}

/// Paired with a `DropSignalSend`, completes when the other end is dropped.
///
/// May be polled any number of times, even after completion.
pub struct DropSignalRecv {
    recv: oneshot::Receiver<()>,
    dead: bool,
}

impl Drop for DropSignalSend {
    fn drop(&mut self) {
        self.send();
    }
}

impl DropSignalSend {
    pub fn send(&mut self) {
        if let Some(send) = self.send.take() {
            let _ = send.send(());
        }
    }
}

impl Future for DropSignalRecv {
    type Item = ();
    type Error = Infallible;

    fn poll(&mut self) -> Poll<(), Infallible> {
        if self.dead {
            return Ok(Async::Ready(()));
        }

        Ok(match self.recv.poll() {
            Ok(Async::NotReady) => Async::NotReady,
            Ok(Async::Ready(())) | Err(_) => {
                self.dead = true;
                Async::Ready(())
            },
        })
    }
}

/// Convenience wrapper around `Arc<DropSignalSend>`.
#[derive(Clone)]
pub struct DropSignalArc(pub Arc<DropSignalSend>);

/*
/// Convenience wrapper around `std::sync::Weak<DropSignalSend>`.
#[derive(Clone)]
pub struct DropSignalArcWeak(pub WeakArc<DropSignalSend>);
*/

/// Create a new paired `DropSignalSend` and `DropSignalRecv`.
pub fn drop_signal_channel() -> (DropSignalSend, DropSignalRecv) {
    let (send, recv) = oneshot::channel();
    (
        DropSignalSend { send: Some(send) },
        DropSignalRecv { recv, dead: false },
    )
}

impl DropSignalSend {
    /// Wrap this in a reference counter.
    pub fn arc(self) -> DropSignalArc {
        DropSignalArc(Arc::new(self))
    }
}

/*
impl DropSignalArc {
    /// Downgrade this into a weak reference counter.
    pub fn downgrade(self) -> DropSignalArcWeak {

    }
}
*/