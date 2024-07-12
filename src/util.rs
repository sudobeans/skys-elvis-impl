/// Represents a time (in seconds).
pub type Time = u64;

/// Removes all values in the given vec, and puts them in the returned vec.
pub fn take_all(v: &mut Vec<u8>) -> Vec<u8> {
    let mut result = Vec::new();
    std::mem::swap(&mut result, v);
    result
}

pub trait Machine: std::fmt::Debug {
    /// Creates and starts a Machine.
    /// Index is the number associated with the machine in a simulation.
    fn start(index: Index) -> Self
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

use std::cell::RefCell;

use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};

use crate::Index;

pub type Ref<T> = &'static RefCell<T>;
pub type Buf = Ref<Vec<u8>>;

#[derive(Clone)]
pub struct Channel {
    pub(super) inbox: Buf,
    pub(super) outbox: Buf,
    us: Index,
    them: Index,
}

impl Channel {
    /// Creates a new channel, and a ref to a bool
    /// that is set to true when a message is sent.
    pub fn new(idx1: Index, idx2: Index) -> (Channel, Channel) {
        let end1 = Box::leak(Box::new(Default::default()));
        let end2 = Box::leak(Box::new(Default::default()));
        (
            Channel {
                inbox: end1,
                outbox: end2,
                us: idx1,
                them: idx2,
            },
            Channel {
                inbox: end2,
                outbox: end1,
                us: idx2,
                them: idx1,
            },
        )
    }

    /// Sends the data to the other end of the channel.
    pub fn send(&mut self, data: &[u8]) {
        self.outbox.borrow_mut().extend_from_slice(data);
    }

    /// Receives the data from this channel.
    pub fn recv(&mut self) -> Vec<u8> {
        take_all(self.inbox.borrow_mut().as_mut())
    }

    /// Returns the index associated with this channel.
    pub fn index(&self) -> usize {
        self.us
    }

    /// Returns the index associated with the other end of this channel.
    pub fn other_index(&self) -> usize {
        self.them
    }
}

// Implementing Device trait for Channel
impl Device for Channel {
    // As it turns out, the Channel struct has all the information needed
    // to be a RxToken and TxToken
    type RxToken<'a> = Channel
    where
        Self: 'a;

    type TxToken<'a> = Channel
    where
        Self: 'a;

    fn receive(
        &mut self,
        _timestamp: smoltcp::time::Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        Some((self.clone(), self.clone()))
    }

    fn transmit(&mut self, _timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        Some(self.clone())
    }

    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        use smoltcp::phy::Checksum::Both;
        use smoltcp::phy::ChecksumCapabilities;

        let mut result = DeviceCapabilities::default();
        result.medium = Medium::Ethernet;
        result.max_transmission_unit = 1500;
        result.max_burst_size = None;
        // device checks no packets,
        // the smoltcp stack has to do it
        result.checksum = ChecksumCapabilities::default();
        result.checksum.ipv4 = Both;
        result.checksum.udp = Both;
        result.checksum.tcp = Both;
        result.checksum.icmpv4 = Both;
        result.checksum.icmpv6 = Both;
        result
    }
}

impl RxToken for Channel {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut incoming = self.recv();
        f(&mut incoming)
    }
}

impl TxToken for Channel {
    fn consume<R, F>(mut self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut outgoing = Vec::with_capacity(len);
        let r = f(&mut outgoing);
        self.send(&outgoing);
        r
    }
}
