use std::{borrow::BorrowMut, cell::RefCell, collections::{BTreeSet, BinaryHeap}, sync::{Arc, Mutex, RwLock}};

use smoltcp::time::Instant;

use crate::{util::{BoxCallback, Callback, Machine, Time}, Index};

/// An event is a function that gets called at a certain time on a node.
pub struct Event<N> {
    pub time: Time,
    pub event: Box<dyn Callback<N>>,
}

/// Internal struct used by the Simulator.
struct ErasedEvent {
    pub time: Time,
    pub event: BoxCallback,
}

impl ErasedEvent {
    fn new<N>(receiver: Arc<Mutex<N>>, event: Event<N>) -> ErasedEvent {
        let f = move || {
            (event.event)(receiver.lock().unwrap().borrow_mut())
        };
        ErasedEvent {
            time: event.time,
            event: Box::new(f),
        }
    }
}

impl<N> PartialEq for Event<N> {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl<N> Eq for Event<N> {}

impl<N> PartialOrd for Event<N> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

impl<N> Ord for Event<N> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

impl<N> std::fmt::Debug for Event<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event").field("time", &self.time).field("desc", &self.desc).finish()
    }
}

impl<N> Event<N> {
    /// Creates an event from the given time and callback.
    pub fn new(time: Time, cb: impl Callback<N>) -> Event<N> {
        Event {
            time,
            event: Box::new(cb),
        }
    }
}

struct Simulator {
    /// The event queue.
    events: Mutex<BinaryHeap<Event>>,
    /// The current time.
    time: RwLock<Time>,
}

impl Simulator {
    pub fn new() -> Simulator {
        Simulator {
            events: Mutex::new(BinaryHeap::new()),
            time: RwLock::new(0),
        }
    }

    /// Runs the next event in the simulator.
    /// Only one event will run at a time.
    pub fn next_event(&self) {
        let mut events_lock = self.events.lock().unwrap();
        let event = events_lock.pop();
        if let Some(event) = event {
            *self.time.write().unwrap() = event.time;
            drop(events_lock);
            (event.event)();
        }
    }
    
    /// Adds an event to the simulator.
    /// Panics if the event is set to take place before the current time.
    pub fn add_event(&self, event: Event) {
        assert!(self.get_time() <= event.time, "event should not be scheduled for the past");
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

lazy_static::lazy_static! {
    pub static ref SIM: Simulator = Simulator::new();
}

/// A Node is just an object that is scheduled separately from other nodes.
trait Node {
    fn run_cb(&mut self, event: impl Callback<Self>) -> Vec<Event<Self>> where Self: Sized {
        event(self)
    }

    /// A type-erased version of run_cb
    fn _run_event(&mut self, )
}

