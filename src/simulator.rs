use std::{borrow::BorrowMut, cell::RefCell, collections::{BTreeSet, BinaryHeap}, sync::{Arc, Mutex, RwLock}};

use smoltcp::time::Instant;

use crate::{util::{BoxCallback, Callback, ReceiveCallback, Machine, Node, Time}, Index};

pub type Msg = Vec<u8>;

pub type Events<N> = Vec<Event<N>>;

/// A node is a thing in a simulation that is separated from other nodes.
/// It could represent a machine on a network, or anything at all.
/// 
/// Using this trait can be a little awkward, since it requires your
/// entire node to be a state machine. For ease of use, consider using the
/// [`SchedulerNode`] trait.
pub trait Node {
    /// Tells the node the current time, so it can act accordingly.
    /// The node should return a vec of messages it wants to send out, along with
    /// the indices of each node that will receive the message.
    /// 
    /// # Panics
    /// 
    /// Nodes are allowed to panic if the time passed in
    /// is less than the last one passed to `poll` or `receive`.
    /// (Basically, nodes can't go back in time.)
    fn poll(&mut self, time: Time) -> Vec<(Index, Msg)>;

    /// Returns the next time this machine should be polled.
    fn poll_at(&self) -> Option<Time>;

    /// Called when this node receives a message.
    /// 
    /// # Parameters
    /// 
    /// * `time` - the time that the node receives the message.
    /// 
    /// * `sender` - the index of the node that sent the message.
    /// 
    /// * `message` - the message this node is receiving.
    /// 
    /// # Panics
    /// 
    /// Nodes are allowed to panic if the time passed in
    /// is less than the last one passed to `poll` or `receive`.
    /// (Basically, nodes can't go back in time.)
    fn receive(&mut self, time: Time, sender: Index, message: Msg);
}

/// A node that has a scheduler.
/// Instead of working directly with `poll` and `poll_at`, you can use
/// the [`Scheduler`] struct to schedule events.
pub trait SchedulerNode {
    /// Returns a reference to the scheduler stored in this node.
    fn scheduler(&mut self) -> &mut Scheduler<Self>;

    /// See [`Node::receive`].
    fn receive(&mut self, time: Time, sender: Index, message: Msg);
}

impl<N: SchedulerNode> Node for N {

} 

/// A scheduler used to schedule events for a node `N`.
pub struct Scheduler<N: ?Sized> {
    events: BinaryHeap<Event<N>>,
}

impl<N> Scheduler<N> {
    /// Add an event to this scheduler.
    /// An event is just a callback that is called at a certain time.
    fn add_event(&mut self, time: Time, cb: Callback<N>);

    /// Give the scheduler the current time.
    /// It will run all events that occurred before that time.
    fn poll(&mut self, time: Time)
}

/// A callback on a node that returns the messages it is sending out
/// as a response.
pub trait Callback<N> {
    /// Call the callback on the given node.
    fn call(self, node: &mut N) -> Vec<(Index, Msg)>;
}

impl<F, N> Callback<N> for F where F: FnOnce(&mut N) -> Vec<(Index, Msg)> {
    fn call(self, node: &mut N) -> Vec<(Index, Msg)> {
        (self)(node)
    }
}

/// An event is just a time bundled with a callback that should be called at that time.
pub struct Event<N: ?Sized>(Time, Box<dyn Callback<N>>);

impl<N> Event<N> {
    /// Creates a new event from a time and a callback.
    pub fn new(time: Time, cb: impl Callback<N>) -> Event<N> {
        Event(time, Box::new(cb))
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
