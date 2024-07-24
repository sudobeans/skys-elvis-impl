use smoltcp::{
    iface::{Interface, SocketHandle, SocketSet},
    phy::{Device, RxToken, TxToken},
    socket::{tcp, AnySocket},
    storage::RingBuffer,
    time::Instant,
    wire::{EthernetAddress, HardwareAddress, IpEndpoint, IpListenEndpoint},
};

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
};

use crate::simulator::{Index, Msg, Node, PollResult, Time};

#[derive(Default)]
struct ElvOsDevice {
    incoming: VecDeque<Msg>,
    outgoing: Vec<Msg>,
}

impl Device for ElvOsDevice {
    type RxToken<'a> = ElvOsRxToken<'a>
    where
        Self: 'a;

    type TxToken<'a> = ElvOsTxToken<'a>
    where
        Self: 'a;

    fn receive(
        &mut self,
        _timestamp: smoltcp::time::Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        if !self.incoming.is_empty() {
            Some((
                ElvOsRxToken(&mut self.incoming),
                ElvOsTxToken(&mut self.outgoing),
            ))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        Some(ElvOsTxToken(&mut self.outgoing))
    }

    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        use smoltcp::phy::Checksum::Both;
        use smoltcp::phy::*;

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

struct ElvOsRxToken<'a>(&'a mut VecDeque<Msg>);

impl<'a> RxToken for ElvOsRxToken<'a> {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut first = self.0.pop_front().expect("queue should not be empty");
        let r = f(first.as_mut_slice());
        r
    }
}

struct ElvOsTxToken<'a>(&'a mut Vec<Msg>);

impl<'a> TxToken for ElvOsTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut msg = vec![0; len];
        let r = f(msg.as_mut_slice());
        self.0.push(msg);
        r
    }
}

pub struct ElvOs {
    events: BinaryHeap<Event>,
    /// The "device" used to do sending and receiving.
    device: ElvOsDevice,
    interface: Interface,
    sockets: SocketSet<'static>,
    /// Extra data associated with each socket
    /// (callbacks)
    socket_data: HashMap<SocketHandle, SocketData>,
    receiver: Index,
    /// The current time on this machine
    time: Time,
}

impl ElvOs {
    pub fn new(time: Time, receiver: Index, hardware_addr: EthernetAddress) -> ElvOs {
        use smoltcp::iface::Config;

        let config = Config::new(HardwareAddress::Ethernet(hardware_addr));
        let mut device = ElvOsDevice::default();
        let interface = Interface::new(config, &mut device, Instant::from_micros(time));
        ElvOs {
            events: BinaryHeap::new(),
            device,
            interface,
            sockets: SocketSet::new(Vec::new()),
            socket_data: HashMap::new(),
            receiver,
            time,
        }
    }

    /// Returns a Socket and its associated SocketData.
    /// Panics if the handle is invalid.
    fn get_sock(&mut self, sock: SocketHandle) -> (&mut tcp::Socket<'static>, &mut SocketData) {
        let socket = self.sockets.get_mut(sock);
        let socket_data = self
            .socket_data
            .get_mut(&sock)
            .expect("failed to get socket data");

        (socket, socket_data)
    }

    fn socket(&mut self) -> SocketHandle {
        let snd = RingBuffer::new(Vec::with_capacity(1500));
        let rcv = RingBuffer::new(Vec::with_capacity(1500));
        self.sockets.add(tcp::Socket::new(rcv, snd))
    }

    fn connect(
        &mut self,
        sock: SocketHandle,
        local_endpoint: impl Into<IpListenEndpoint>,
        remote_endpoint: impl Into<IpEndpoint>,
    ) {
        let sock = self.sockets.get_mut::<tcp::Socket>(sock);
        sock.connect(self.interface.context(), remote_endpoint, local_endpoint)
            .unwrap();
    }

    /// Called when a connection is created between this sock and another.
    /// This could be from either a [`listen`](ElvOs::listen)
    /// or [`connect`](ElvOs::connect) call.
    fn set_connect_callback(&mut self, sock: SocketHandle, cb: fn(&mut ElvOs)) {
        let sock_data = self.get_sock(sock).1;
        sock_data.connect = cb;
    }

    fn listen(&mut self, sock: SocketHandle, local_endpoint: impl Into<IpListenEndpoint>) {
        let sock = self.get_sock(sock).0;
        sock.listen(local_endpoint);
    }

    fn send(&mut self, sock: SocketHandle, msg: &[u8]) -> std::io::Result<usize> {
        let sock = self.get_sock(sock).0;
        sock.send_slice(msg)
            .or(Err(std::io::ErrorKind::NotConnected.into()))
    }

    fn set_recv_callback(&mut self, sock: SocketHandle, cb: fn(&mut ElvOs)) {
        let sock_data = self.get_sock(sock).1;
        sock_data.recv = cb;
    }

    fn recv(&mut self, sock: SocketHandle) -> Msg {
        let sock = self.get_sock(sock).0;
        receive_all(sock)
    }

    /// Returns an iterator over the TCP sockets
    fn socks(&mut self) -> impl Iterator<Item = &mut tcp::Socket<'static>> {
        self.sockets.iter_mut().map(|(_handle, sock)| {
            tcp::Socket::downcast_mut(sock).expect("should only be storing tcp sockets")
        })
    }
}

/// downcasts a generic tcp socket to an ordinary socket
fn downcast<'a>(sock: &'a mut smoltcp::socket::Socket<'static>) -> &'a mut tcp::Socket<'static> {
    tcp::Socket::downcast_mut(sock).expect("should be a TCP socket")
}

/// Receives all data from a smoltcp socket buffer and puts it in a msg.
fn receive_all(sock: &mut tcp::Socket<'static>) -> Msg {
    let mut result = vec![0; sock.recv_capacity()];
    let mut start = 0;
    loop {
        // stop when there's an error or no more data is received
        match sock.recv_slice(&mut result[start..]) {
            Ok(0) => break,
            Err(e) => panic!("(this is a demo i can panic with {e})"),
            Ok(num) => start = num,
        }
    }
    result
}

impl Node for ElvOs {
    fn poll(&mut self, time: Time, incoming: PollResult) -> PollResult {
        use smoltcp::socket::tcp::State::*;

        self.time = time;

        // save the state of the sockets (so we'll know to make the
        // listen and connect callbacks)
        let mut connecting_socks: HashSet<SocketHandle> = HashSet::new();
        for (handle, sock) in self.sockets.iter_mut() {
            let sock = downcast(sock);
            match sock.state() {
                Listen | SynSent | SynReceived => {
                    connecting_socks.insert(handle);
                }
                _other => {}
            }
        }

        // receive incoming
        self.device
            .incoming
            .extend(incoming.into_iter().map(|(_index, msg)| msg));
        // poll smoltcp
        self.interface.poll(
            Instant::from_micros(time),
            &mut self.device,
            &mut self.sockets,
        );

        // make connect and receive callbacks
        let handles = Vec::from_iter(self.sockets.iter().map(|(handle, _sock)| handle));
        for handle in handles {
            let (socket, data) = self.get_sock(handle);
            let data = *data;
            let can_recv = socket.can_recv();

            if socket.is_open() && connecting_socks.contains(&handle) {
                (data.connect)(self)
            }

            if can_recv {
                (data.recv)(self)
            }
        }

        // run functions in scheduler
        while let Some(Event(event_time, _)) = self.events.peek() {
            if *event_time < time {
                let ev = self.events.pop().unwrap();
                (ev.1)(self);
            } else {
                break;
            }
        }

        // send outgoing data
        let outgoing = take_all(&mut self.device.outgoing);
        Vec::from_iter(outgoing.into_iter().map(|msg| (self.receiver, msg)))
    }

    fn poll_at(&mut self) -> Option<Time> {
        let smoltcp_poll_time = self
            .interface
            .poll_at(Instant::from_micros(self.time), &mut self.sockets);
        let smoltcp_poll_time = smoltcp_poll_time.map(|time| time.total_micros());
        let events_poll_time = self.events.peek().map(|event| event.0);

        // choose earliest of 2 times
        match (smoltcp_poll_time, events_poll_time) {
            (Some(t), None) => Some(t),
            (None, Some(t)) => Some(t),
            (Some(t1), Some(t2)) => Some(Time::min(t1, t2)),
            (None, None) => None,
        }
    }
}

type BoxCallback = fn(&mut ElvOs);

#[derive(Clone, Copy)]
struct SocketData {
    /// Callbacks, set by `set_connect_callback`, etc.
    connect: BoxCallback,
    recv: BoxCallback,
}

impl Default for SocketData {
    fn default() -> Self {
        fn nothing(_: &mut ElvOs) {}
        Self {
            connect: nothing,
            recv: nothing,
        }
    }
}

/// Removes all values in the given vec, and puts them in the returned vec.
pub fn take_all(v: &mut Vec<Msg>) -> Vec<Msg> {
    let mut result = Vec::new();
    std::mem::swap(&mut result, v);
    result
}

/// An event is just a function and the time it gets called.
/// Ordered so that the earliest events come first in Rust's BinaryHeap.
struct Event(Time, fn(&mut ElvOs));

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for Event {}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0).reverse()
    }
}
