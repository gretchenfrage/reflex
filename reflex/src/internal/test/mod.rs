
extern crate tokio;
extern crate pretty_env_logger;

use crate::*;
use crate::msg_union::*;
use crate::internal::*;
use tokio::prelude::*;

fn init_log() {
    use std::sync::Once;

    static INIT_LOG: Once = Once::new();
    INIT_LOG.call_once(|| {
        pretty_env_logger::init();
    })
}

#[test]
fn test_a() {
    // very simple use test of internal dispatching

    init_log();

    use smallvec::smallvec;

    tokio::run(future::lazy(|| {
        struct Foo;

        impl Actor for Foo {
            type Message = ();

            type End = ();

            type SubordinateEnd = ();
        }

        let actor = Foo;
        let (
            state,
            mut send,
        ) = create::create_actor(actor);

        tokio::spawn(state);

        send.try_send(ActorMessage::Shared(smallvec![(), (), ()])).unwrap();
        send.try_send(ActorMessage::Mut(())).unwrap();

        Ok(())
    }));
}

#[test]
fn test_b() {
    // log test which involves crossing thread boundaries, and verifying
    // synchronization using an RwLock

    init_log();

    use std::sync::RwLock;
    use std::mem::drop;
    use smallvec::smallvec;
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    tokio::run(future::lazy(|| {
        struct Foo(RwLock<()>);

        impl Actor for Foo {
            type Message = FooUnion;

            type End = ();

            type SubordinateEnd = ();
        }

        struct FooShared;
        struct FooMut;
        #[derive(Copy, Clone)]
        struct FooUnion;

        impl MessageUnionShared<Foo> for FooShared {
            fn process(self, actor: ActorGuardShared<Foo>) {
                spawn(move || {
                    debug!("+ acq shared");
                    let lock = actor.0.try_read().unwrap();
                    sleep(Duration::from_secs(1));
                    drop(lock);
                    debug!("- rel shared");
                });
            }
        }

        impl MessageUnionMut<Foo> for FooMut {
            fn process(self, actor: ActorGuardMut<Foo>) {
                spawn(move || {
                    debug!("+ acq mut");
                    let lock = actor.0.try_write().unwrap();
                    sleep(Duration::from_secs(1));
                    drop(lock);
                    debug!("- rel mut");
                });
            }
        }

        impl MessageUnion<Foo> for FooUnion {
            type Shared = FooShared;
            type Mut = FooMut;
        }

        let actor = Foo(RwLock::new(()));
        let (
            state,
            mut send,
        ) = create::create_actor(actor);

        tokio::spawn(state);

        send.try_send(ActorMessage::Shared(smallvec![FooShared, FooShared, FooShared])).unwrap();
        send.try_send(ActorMessage::Mut(FooMut)).unwrap();
        send.try_send(ActorMessage::Shared(smallvec![FooShared])).unwrap();

        Ok(())
    }));
}