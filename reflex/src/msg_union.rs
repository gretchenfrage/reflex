
use smallvec::SmallVec;

/// Types which can be valid messages.
///
/// Currently a blanket-impl for anything `Send` and `'static`.
pub trait Message: Send + 'static {}

impl<T: Send + 'static> Message for T {}

/// Trait binding together a set of message types which an actor can process,
/// both shared and mut.
///
/// Implementations are meant to be created with macros.
///
/// This type is never meant to be instantiated.
pub trait MessageTypeUnion: Sized + Send + Sync + Copy + 'static {
    /// Associated shared message union type.
    type SharedUnion: Message;

    /// Associated mut message union type.
    type MutUnion: Message;
}

/// The runtime value which can be sent into a mailbox.
pub enum MailboxEntry<T: MessageTypeUnion> {
    /// The shared `MailboxEntry` variant may store several messages in a `SmallVec`.
    ///
    /// Since processing a shared message can, by-definition, occur concurrently, this has
    /// little effect on code complexity. however, this allows message producers to release
    /// their messages in an atomic batch, which prevents concurrency-unfriendly interleaving
    /// of those messages with exclusively processed messages from another producer.
    Shared(SmallVec<[<T as MessageTypeUnion>::SharedUnion; 4]>),
    Mut(<T as MessageTypeUnion>::MutUnion),
}

/// Implement MessageUnion for (), to help partially-written code compile.
impl MessageTypeUnion for () {
    type SharedUnion = ();
    type MutUnion = ();
}