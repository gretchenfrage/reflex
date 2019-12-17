
## reflex

> what?

a tree of actor-like subroutines. 

> why?

to organize my code which lives close to the internet, and was already following
these patterns, but tediously.

> how?

simple and fast, by virtue of little code with little abstraction.

--- 

What's an actor?

- an actor is a struct that implements `Actor`
- an actor knows how to process some message types
- an actor might supervise other actors
- an actor has a supervisor of its own

What's up with message processing?

- messages are structs
- actors process messages by implementing `ReactShared<Msg>` or `ReactMut<Msg>`
- a `Mailbox` is a message sender for an actor

#### backpressure

Message queues are finite in size. If an actor receives messages faster than it 
can process them, the mailbox can get backed up, and attempts to put new messages
in the queue can block.

This feature can be used to regulate data flow, but it can also open up services 
up to denial-of-service attacks depending on how it's employed.

TODO: examples on backpressure