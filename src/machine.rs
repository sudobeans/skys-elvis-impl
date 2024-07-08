/// Represents a time (in seconds).
pub type Time = u64;

pub trait Machine: std::fmt::Display {
    /// Creates and starts a Machine.
    fn start() -> Self
    where
        Self: Sized;

    /// Adds a connection to this machine.
    fn add_connection(&mut self, channel: Channel, time: Time);

    /// Tells the machine the current time.
    /// The machine might also do processing when poll is called,
    /// such as handling received messages, sending new messages,
    /// and updating state.
    fn poll(&mut self, time: Time);

    /// Get the next time the machine should be polled.
    /// If time is none, polling this is not necessary.
    /// The result of this method may change due to external events,
    /// such as being polled, receiving a message, or having a connection added.
    /// For example, this method could return 50 if the sim is supposed to do
    /// something at the 50th second.
    fn poll_at(&self) -> Option<Time>;
}

use std::{cell::RefCell, collections::VecDeque, rc::Rc, sync::mpsc::*};

use crate::Index;

pub type Message<'a> = &'a [u8];
pub type Buf = Rc<RefCell<VecDeque<u8>>>;

pub struct Channel2 {
    inbox: Buf,
    outbox: Buf,
}

impl Channel2 {
    
}
