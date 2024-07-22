/// Represents a time (in seconds).
pub type Time = i64;

/// Removes all values in the given vec, and puts them in the returned vec.
pub fn take_all(v: &mut Vec<u8>) -> Vec<u8> {
    let mut result = Vec::new();
    std::mem::swap(&mut result, v);
    result
}

pub trait Machine: std::fmt::Debug {
    /// starts the Machine.
    fn start(&self)
    where
        Self: Sized;

    /// Adds a connection to this machine.
    fn add_connection(&mut self, channel: Channel);

    /// The machine receives the given slice of bytes.
    fn receive(&self, msg: &[u8]);
}

use std::{cell::RefCell, rc::{Rc, Weak}};

use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};

use crate::{simulator::Event, Index};

pub type Ref<T> = &'static RefCell<T>;
pub type Buf = Ref<Vec<u8>>;

pub struct Channel {
    pub(super) receiver: Weak<dyn Machine>,
}

impl Channel {
    pub fn send(&self, buf: &[u8]) {
        if let Some(machine) = self.receiver.upgrade() {
            machine.receive(buf);
        }
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

pub trait Callback<T>: FnOnce(&mut T) -> Vec<Event<T>> + 'static + Send {}

pub type BoxCallback = Box<dyn FnOnce() + 'static + Send>;

pub trait CallbackMut<T>: FnMut(&mut T) -> Vec<Event<T>> + 'static + Send {}
