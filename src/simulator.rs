use std::{cell::RefCell, collections::{BTreeSet, BinaryHeap}, sync::{Mutex, RwLock}};

use smoltcp::time::Instant;

use crate::{util::{Callback, Machine, Time}, Index};

/// An event is simply a function that gets called at a certain time.
pub struct Event {
    time: Time,
    event: Box<dyn Callback>,
    desc: &'static str,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl Eq for Event {}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event").field("time", &self.time).field("desc", &self.desc).finish()
    }
}

impl Event {
    pub fn new(time: Time, cb: impl Callback) -> Event {
        Event {
            time,
            event: Box::new(cb),
            desc: "default"
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
