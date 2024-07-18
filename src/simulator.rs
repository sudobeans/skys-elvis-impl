use std::{cell::RefCell, collections::{BTreeSet, BinaryHeap}, sync::Mutex};

use crate::{util::{Machine, Time}, Index};

/// An event is simply a function that gets called at a certain time.
struct Event {
    time: Time,
    event: Box<dyn FnOnce() + 'static>,
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

struct Simulator {
    /// The event queue.
    events: BinaryHeap<Event>,
    /// The current time.
    time: Time,
}

impl Simulator {
    fn new() -> Simulator {
        Simulator {
            events: BinaryHeap::new(),
            time: 0
        }
    }
}

static SIM: Mutex<Simulator> = Mutex::new(Simulator::new());
