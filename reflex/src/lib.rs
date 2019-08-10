
/// Actor types.
pub trait Actor: Sized + Send + Sync + 'static {
    /// Union type for messages which this actor can process.
    type Message: MessageUnion<Self>;
}

/// Actor types which can process a particular message type with `&self`.
pub trait ReactShared<Msg>: Actor {
    // TODO: fn process with actor guard
}

/// Actor types which can process a particular message type with `&mut self`.
pub trait ReactMut<Msg>: Actor {
    // TODO: fn process with actor guard
}

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

/// Trait binding together a set of message for which an actor implements `ReactShared`.
///
/// #### Type Parameters
/// * `Act` - the actor type which processes this message.
pub trait MessageUnionShared<Act>: Message {
    // TODO: fn process with actor guard
}

/// Trait binding together a set of message for which an actor implements `ReactMut`.
///
/// #### Type Parameters
/// * `Act` - the actor type which processes this message.
pub trait MessageUnionMut<Act>: Message {
    // TODO: fn process with actor guard
}

/// Types which can be valid messages.
///
/// Implementations are meant to be created with macros.
///
/// Currently a blanket-impl for anything `Send` and `'static`.
pub trait Message: Send + 'static {}

impl<T: Send + 'static> Message for T {}
