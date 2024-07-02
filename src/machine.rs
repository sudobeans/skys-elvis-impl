/// Represents a time (in seconds).
pub type Time = u64;

pub trait Machine: std::fmt::Display {
    /// Creates and starts a Machine.
    fn start(name: String) -> Self
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
    fn poll_at(&mut self) -> Option<Time>;
}

use std::sync::mpsc::*;

pub type Message = Vec<u8>;

/// Two-way channel representing a connection between 2 machines.
pub struct Channel {
    send: Sender<Message>,
    recv: Receiver<Message>,

    /// Our name
    us: String,
    /// other end's name
    them: String,

    /// this channel is also sent on
    /// when a message is sent to indicate update
    updated: Sender<()>,
}

impl Channel {
    /// Creates 2 connected channel endpoints,
    /// and a flag for detecting when messages are received.
    pub fn new(name1: String, name2: String) -> (Channel, Channel, Receiver<()>) {
        let (s1, r1) = channel();
        let (s2, r2) = channel();
        let (updated, updated_recv) = channel();

        (
            Channel { us: name1, them: name2, send: s1, recv: r2, updated: updated.clone() },
            Channel { us: name2, them: name1, send: s2, recv: r1, updated },
            updated_recv
        )
    }

    pub fn receiver_name(&self) -> String {
        self.them.clone()
    }

    pub fn sender_name(&self) -> String {
        self.us.clone()
    }

    pub fn send(&mut self, message: Message) {
        let _ = self.send.send(message);
    }

    pub fn recv(&mut self) -> Option<Message> {
        self.recv.try_recv().ok()
    }
}

