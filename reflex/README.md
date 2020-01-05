
## reflex

> what?

 tree of actor-like subroutines. 

> why?

to organize my code which lives close to the internet, and was already following
these patterns, but tediously.

> how?

simple and fast, by virtue of little code with little abstraction.

--- 

What's an actor?

- an actor is a struct that implements `Actor`
- an actor knows how to process some message types
- an actor can supervise other actors
- an actor has a supervisor of its own

What's message processing?

- messages are structs
- actors process messages by implementing `ReactShared<Msg>` or `ReactMut<Msg>`
- a `Mailbox` is a handle to send messages to an actor
- when processing a message, an actor can:
    - mutate itself
    - terminate itself
    - spawn subordinate actors
    - send more messages

### management



### backpressure

Message queues are finite in size. If an actor receives messages faster than it 
can process them, the mailbox can get backed up, and attempts to put new messages
in the queue can block.

This feature can be used to regulate data flow, but it can also open up services 
up to denial-of-service attacks depending on how it's employed. For example, consider
a streaming services where a server produces data in real-time, and a series of 
clients subscribe to it.

```
  +--------------+
  | server actor |
  +--------------+
      |
      |-----|-----|     \
     +-+   +-+   +-+    |<--- client connection
     +-+   +-+   +-+    /     subordinate actors
      |     |     |
~~~~~~|~~~~~|~~~~~|~~~~~~ TCP internet connections
      |     |     |
      V     V     V
    +===+ +===+ +===+   \
    ‖ ! ‖ ‖ ! ‖ ‖ ! ‖   |<--- untrusted clients
    +===+ +===+ +===+   /
```

##### Vulnerable Approach

The server actor sends data packets as messages to the client actors. These actors
forward the packets to a TCP sink.

> what's the problem?

A client, maliciously or not, throttles its TCP connection. Back pressure occurs 
on the server side of the TCP connection, since the connection actor is receiving
packets faster than it can offload them. Backpressure propagates up through the
connection actor's mailbox, and blocks the server actor when it tries to send in
a new message. 

In summary, a single client's unresponsiveness has made the server unresponsive
for all clients.

> how to fix it?

An untrusted agent should not be responsible for relieving a backpressure pathway 
that propagates to any services that is not exclusively for that client, such
as services for other clients, or for the server itself.

