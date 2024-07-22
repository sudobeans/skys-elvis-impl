use smoltcp::{
    iface::{Interface, SocketHandle, SocketSet}, phy::{Device, RxToken, TxToken}, socket::tcp, storage::RingBuffer, wire::{IpEndpoint, IpListenEndpoint}
};

use crate::{simulator::{Event, SIM}, util::{Callback, CallbackMut, Channel, Time}};

use std::{cell::RefCell, collections::{HashMap, VecDeque}, sync::Arc};

type Msg = Vec<u8>;

struct ElvOsDevice {
    incoming: VecDeque<Msg>,
    outgoing: VecDeque<Msg>,
}

impl Device for ElvOsDevice {
    type RxToken<'a> = ElvOsRxToken<'a>
    where
        Self: 'a;

    type TxToken<'a> = ElvOsTxToken<'a>
    where
        Self: 'a;

    fn receive(&mut self, _timestamp: smoltcp::time::Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        if !self.incoming.is_empty() {
            Some((ElvOsRxToken(&mut self.incoming), ElvOsTxToken(&mut self.outgoing)))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        Some(ElvOsTxToken(&mut self.outgoing))
    }

    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        todo!()
    }
}

struct ElvOsRxToken<'a>(&'a mut VecDeque<Msg>);

impl<'a> RxToken for ElvOsRxToken<'a> {
    fn consume<R, F>(self, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R {
        let mut first = self.0.pop_front().expect("queue should not be empty");
        let r = f(first.as_mut_slice());
        r
    }
}

struct ElvOsTxToken<'a>(&'a mut VecDeque<Msg>);

impl<'a> TxToken for ElvOsTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R {
        let mut msg = vec![0; len];
        let r = f(msg.as_mut_slice());
        self.0.push_back(msg);
        r
    }
}

pub struct ElvOs(RefCell<ElvOsInner>);

struct ElvOsInner {
    /// The "device" used to do sending and receiving.
    device: ElvOsDevice,
    interface: Interface,
    sockets: SocketSet<'static>,
    /// Extra data associated with each socket
    /// (callbacks)
    socket_data: HashMap<SocketHandle, SocketData>,
}

impl ElvOsInner {

    /// Polls the inner smoltcp
    fn poll(&mut self) {
        self.interface.poll(SIM.get_instant(), &mut self.device, &mut self.sockets);
    }

    /// Returns a Socket and its associated SocketData.
    /// Panics if the handle is invalid.
    fn get_sock(&mut self, sock: SocketHandle) -> (&mut tcp::Socket<'static>, &mut SocketData) {
        let socket = self.sockets.get_mut(sock);
        let socket_data = self.socket_data.get_mut(&sock).expect("failed to get socket data");
        
        (socket, socket_data)
    }

    fn socket(&mut self) -> SocketHandle {
        let snd = RingBuffer::new(Vec::with_capacity(1500));
        let rcv = RingBuffer::new(Vec::with_capacity(1500));
        self.sockets.add(tcp::Socket::new(rcv, snd))
    }

    fn connect(&mut self, sock: SocketHandle, local_endpoint: impl Into<IpListenEndpoint>, remote_endpoint: impl Into<IpEndpoint>) {
        let sock = self.sockets.get_mut::<tcp::Socket>(sock);
        sock.connect(self.interface.context(), remote_endpoint, local_endpoint).unwrap();
    }

    fn set_connect_callback(&mut self, sock: SocketHandle, cb: impl CallbackMut) {
        let sock_data = self.get_sock(sock).1;
        sock_data.connect = Box::new(cb);
    }

    fn listen(&mut self, sock: SocketHandle, local_endpoint: impl Into<IpListenEndpoint>) {
        let sock = self.get_sock(sock).0;
        sock.listen(local_endpoint);
    }
    
    fn set_listen_callback(&mut self, sock: SocketHandle, cb: impl CallbackMut) {
        let sock_data = self.get_sock(sock).1;
        sock_data.listen = Box::new(cb);
    }

    fn send(&mut self, sock: SocketHandle, msg: &[u8]) -> std::io::Result<usize> {
        let sock = self.get_sock(sock).0;
        sock.send_slice(msg).or(Err(std::io::ErrorKind::NotConnected.into()))
    }

    fn set_recv_callback(&mut self, sock: SocketHandle, cb: impl CallbackMut) {
        let sock_data = self.get_sock(sock).1;
        sock_data.recv = Box::new(cb);
    }

    fn recv(&mut self, sock: SocketHandle, msg: &mut[u8]) -> std::io::Result<usize> {
        let sock = self.get_sock(sock).0;
        sock.recv_slice(msg).or(Err(std::io::ErrorKind::NotConnected.into()))
    }

    fn incoming_packet(&mut self, msg: Msg) {
        self.device.incoming.push_back(msg);

        // poll!
        self.interface.poll(SIM.get_instant(), &mut self.device, &mut self.sockets);

        // make sure packets were received
        assert!(self.device.incoming.is_empty());
    }

    /// Sends outgoing packets through channel
    fn outgoing_packets(&mut self) -> VecDeque<Msg> {
        
    }
}

type BoxCallback = Box<dyn FnMut()>;

struct SocketData {
    /// Callbacks, set by `set_connect_callback`, etc.
    connect: BoxCallback,
    listen: BoxCallback,
    recv: BoxCallback,
}

impl Default for SocketData {
    fn default() -> Self {
        fn nothing() {}
        let nothing_box = Box::new(nothing);

        SocketData {
            connect: nothing_box.clone(),
            listen: nothing_box.clone(),
            recv: nothing_box,
        }
    }
}
