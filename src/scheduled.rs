/*******
 * THIS FILE IS NOT CURRENTLY PART OF THE SIMULATION
 * I AM PLANNING TO FIX IT UP LATER
 *******/

use std::collections::BinaryHeap;

use crate::simulator::{PollResult, Time};

pub type Events<N> = Vec<Event<N>>;

/// A scheduler used to schedule events for a node `N`.
pub struct Scheduler<N> {
    events: BinaryHeap<Event<N>>,
}

impl<N> Scheduler<N> {
    /// Add an event to this scheduler's queue.
    /// An event is just a callback that is called at a certain time.
    pub fn add_event(&mut self, time: Time, cb: fn(&mut N)) {
        self.events.push(Event(time, cb));
    }

    /// Give the scheduler the current time, and the node.
    /// It will run all events that occurred before or at that time.
    /// Then it will return the messages those events are sending out.
    pub fn poll(&mut self, time: Time, node: &mut N) -> PollResult {
        let mut result = Vec::new();
        while let Some(event) = self.events.peek() {
            if event.0 <= time {
                let event = self.events.pop().unwrap();
                let msgs = (event.1)(node);
                result.append(&mut msgs);
            } else {
                break;
            }
        }
        result
    }

    /// Returns the time earliest event in the queue will occur
    /// (which is the next time poll should be called.)
    /// Returns `None` if there are no events.
    pub fn poll_at(&self) -> Option<Time> {
        self.events.peek().map(|event| event.0)
    }
}

/// An event is just a time bundled with a callback that should be called at that time.
pub struct Event<N>(Time, fn(&mut N));

impl<N> Event<N> {
    /// Creates a new event from a time and a callback.
    pub fn new(time: Time, cb: fn(&mut N)) -> Event<N> {
        Event(time, cb)
    }
}

impl<N> PartialEq for Event<N> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<N> Eq for Event<N> {}

impl<N> PartialOrd for Event<N> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<N> Ord for Event<N> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<N> std::fmt::Debug for Event<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event").field("time", &self.0).field("desc", &self.0).finish()
    }
}
