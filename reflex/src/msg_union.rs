
use crate::{ActorGuardShared, ActorGuardMut};

/// Trait binding together a set of message types which an actor can process,
/// both shared and mut.
///
/// Implementations are meant to be created with macros.
///
/// Currently, this only uses associated types to bundle the shared and mut message
/// union types into a single type parameter.
///
/// #### Type Parameters
/// * `Act` - the actor type which processes this message.
pub trait MessageUnion<Act>: Sized + Send + Sync + Copy + 'static{
    /// Associated shared message union type.
    type Shared: MessageUnionShared<Act>;

    /// Associated mut message union type.
    type Mut: MessageUnionMut<Act>;
}

/// Trait binding together a set of message types for which an actor implements `ReactShared`.
///
/// #### Type Parameters
/// * `Act` - the actor type which processes this message.
pub trait MessageUnionShared<Act>: Sized + Message {
    /// Delegate to `<Act as ReactShared<Msg>>::process`, where `Msg` is our internal runtime
    /// variant, passing our inner value as the message parameter.
    fn process(self, actor: ActorGuardShared<Act>);
}

/// Trait binding together a set of message types for which an actor implements `ReactMut`.
///
/// #### Type Parameters
/// * `Act` - the actor type which processes this message.
pub trait MessageUnionMut<Act>: Sized + Message {
    /// Delegate to `<Act as ReactMut<Msg>>::process`, where `Msg` is our internal runtime
    /// variant, passing our inner value as the message parameter.
    fn process(self, actor: ActorGuardMut<Act>);
}

/// Types which can be valid messages.
///
/// Implementations are meant to be created with macros.
///
/// Currently a blanket-impl for anything `Send` and `'static`.
pub trait Message: Send + 'static {}

impl<T: Send + 'static> Message for T {}
