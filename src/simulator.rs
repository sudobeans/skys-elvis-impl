use std::{borrow::BorrowMut, cell::RefCell, collections::{BTreeSet, BinaryHeap}, sync::{Arc, Mutex, RwLock}};

use smoltcp::time::Instant;

use crate::{util::{BoxCallback, Callback, ReceiveCallback, Machine, Node, Time}, Index};

pub type Msg = Vec<u8>;

pub type Events<N> = Vec<Event<N>>;

pub enum Event<N> {
    /// Sets a callback to occur at the given time.
    Callback {
        time: Time,
        event: Box<dyn Callback<N>>,
    },
    /// Sends a message to the node at the given index.
    Send {
        receiver: Index,
        message: Msg,
    },
    /// Sets the function that should be called when this machine receives a message.
    SetRecv {
        callback: Box<dyn ReceiveCallback<N>>,
    }
}

impl<N> Event<N> {
    pub fn new_callback(time: Time, event: Box<dyn Callback<N>>) -> Event<N> {
        Event::Callback {
            time,
            event,
        }
    }

    pub fn new_send(receiver: Index, message: Msg) -> Event<N> {
        Event::Send {
            receiver,
            message
        }
    }

    pub fn set_recv()
}

/// An event is a function that gets called at a certain time on a node.
pub struct CallbackEvent<N> {
    pub time: Time,
    pub event: Box<dyn Callback<N>>,
}

/// Internal struct used by the Simulator.
struct ErasedEvent {
    pub time: Time,
    // forgive this type.
    // like an Event returns more events, an ErasedEvent returns ErasedEvents
    pub event: Box<dyn FnOnce() -> Vec<ErasedEvent> + 'static + Send>,
}

/// A node with its event queue.
struct NodeWithEvents<N> {
    node: N,
    events: BinaryHeap<CallbackEvent<N>>,
}

/// Trait so we can remove the generic from NodeWithevents
trait NodeWithEventsTrait {
    fn next_event_time(&mut self) -> Option<Time>;

    fn run_event(&mut self);
}

impl<N: Node> NodeWithEventsTrait for NodeWithEvents<N> {
    fn next_event_time(&mut self) -> Option<Time> {
        self.events.peek().map(|event| event.time)
    }
    
    fn run_event(&mut self) {
        if let Some(event) = self.events.pop() {
            let evs = (event.event)(&mut self.node);
            self.events.extend(evs);
        }
    }
}


impl<N> PartialEq for CallbackEvent<N> {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl<N> Eq for CallbackEvent<N> {}

impl<N> PartialOrd for CallbackEvent<N> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

impl<N> Ord for CallbackEvent<N> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

impl<N> std::fmt::Debug for CallbackEvent<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event").field("time", &self.time).field("desc", &self.desc).finish()
    }
}

impl<N> CallbackEvent<N> {
    /// Creates an event from the given time and callback.
    pub fn new(time: Time, cb: impl Callback<N>) -> CallbackEvent<N> {
        CallbackEvent {
            time,
            event: Box::new(cb),
        }
    }
}

struct Simulator {
    /// Nodes.
    nodes: Vec<Box<dyn NodeWithEventsTrait>>,
    /// The current time.
    time: Time,
    /// The list of nodes that had something 
}

impl Simulator {
    pub fn new() -> Simulator {
        Simulator {
            events: BinaryHeap::new(),
            time: 0,
        }
    }

    /// Runs the next event in the simulator.
    /// Only one event will run at a time.
    pub fn next_event(&mut self) {
        if let Some(event) = self.events.pop() {
            self.time = event.time;
            let eraseds = (event.event)();
            self.events.extend(eraseds);
        }
    }
    
    /// Adds a node to the simulator with an initial event.
    pub fn add_node<N: Node>(&mut self, node: N, event: CallbackEvent<N>) {
        assert!(self.get_time() <= event.time, "event should not be scheduled for the past");
        let erased = ErasedEvent::new()
        self.events.lock().unwrap().push(event);
    }

    /// Gets the current time of the sim.
    pub fn get_time(&self) -> Time {
        *self.time.read().unwrap()
    }

    /// Gets the current time of the sim as a smoltcp instant.
    /// (In real ELVIS this would probably not be public.)
    pub fn get_instant(&self) -> Instant {
        Instant::from_millis(self.get_time())
    }
}
