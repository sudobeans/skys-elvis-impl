use smoltcp::{
    iface::{Interface, SocketHandle, SocketSet}, phy::{Device, RxToken, TxToken}, socket::tcp, storage::RingBuffer, wire::{IpEndpoint, IpListenEndpoint}
};

use crate::util::Channel;

use std::{cell::RefCell, collections::HashMap};

trait Os {
    /// Socket identifier, like a file descriptor.
    type SocketId;

    fn socket(&self) -> Self::SocketId;

    fn connect(&self, sock: Self::SocketId, local_endpoint: impl Into<IpListenEndpoint>, remote_endpoint: impl Into<IpEndpoint>);

    fn set_connect_callback(&self, sock: Self::SocketId, cb: impl FnMut() + 'static);

    fn listen(&self, sock: Self::SocketId, local_endpoint: impl Into<IpListenEndpoint>);
    
    fn set_listen_callback(&self, sock: Self::SocketId, cb: impl FnMut() + 'static);

    fn send(&self, sock: Self::SocketId, msg: &[u8]) -> std::io::Result<usize>;

    fn set_recv_callback(&self, sock: Self::SocketId, cb: impl FnMut() + 'static);

    fn recv(&self, sock: Self::SocketId, msg: &mut[u8]) -> std::io::Result<usize>;
}


struct ElvOsDevice {
    /// For sending
    chan: Channel,
    /// For putting received data in
    received: Vec<u8>,
}

impl Device for ElvOsDevice {
    type RxToken<'a> = ElvOsRxToken<'a>
    where
        Self: 'a;

    type TxToken<'a> = ElvOsTxToken<'a>
    where
        Self: 'a;

    fn receive(&mut self, _timestamp: smoltcp::time::Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        if !self.received.is_empty() {
            Some((ElvOsRxToken(&mut self.received), ElvOsTxToken(&mut self.chan)))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        Some(ElvOsTxToken(&mut self.chan))
    }

    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        todo!()
    }
}

struct ElvOsRxToken<'a>(&'a mut Vec<u8>);

impl<'a> RxToken for ElvOsRxToken<'a> {
    fn consume<R, F>(self, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R {
        let r = f(self.0.as_mut_slice());
        self.0.clear();
        r
    }
}

struct ElvOsTxToken<'a>(&'a mut Channel);

impl<'a> TxToken for ElvOsTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R {
        let mut buf = vec![0; len];
        let r = f(buf.as_mut_slice());
        self.0.send(buf.as_slice());
        r
    }
}

struct ElvOs(RefCell<ElvOsInner>);

struct ElvOsInner {
    /// The "device" used to do sending and receiving.
    device: Option<ElvOsDevice>,
    interface: Interface,
    sockets: SocketSet<'static>,
    /// Extra data associated with each socket
    /// (callbacks)
    socket_data: HashMap<SocketHandle, SocketData>,
}

impl ElvOsInner {

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

    fn set_connect_callback(&mut self, sock: SocketHandle, cb: impl FnMut() + 'static) {
        let sock_data = self.get_sock(sock).1;
        sock_data.connect = Box::new(cb);
    }

    fn listen(&mut self, sock: SocketHandle, local_endpoint: impl Into<IpListenEndpoint>) {
        let sock = self.get_sock(sock).0;
        sock.listen(local_endpoint);
    }
    
    fn set_listen_callback(&mut self, sock: SocketHandle, cb: impl FnMut() + 'static) {
        let sock_data = self.get_sock(sock).1;
        sock_data.listen = Box::new(cb);
    }

    fn send(&mut self, sock: SocketHandle, msg: &[u8]) -> std::io::Result<usize> {
        let sock = self.get_sock(sock).0;
        sock.send_slice(msg).or(Err(std::io::ErrorKind::NotConnected.into()))
    }

    fn set_recv_callback(&mut self, sock: SocketHandle, cb: impl FnMut() + 'static) {
        let sock_data = self.get_sock(sock).1;
        sock_data.recv = Box::new(cb);
    }

    fn recv(&mut self, sock: SocketHandle, msg: &mut[u8]) -> std::io::Result<usize> {
        let sock = self.get_sock(sock).0;
        sock.recv_slice(msg).or(Err(std::io::ErrorKind::NotConnected.into()))
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

impl ElvOs {
    /// gets SocketData and Socket object associated with handle, then calls f
    fn with_elvos_inner(&self, f: impl FnOnce(&mut ElvOsInner)){
        let mut elvos_borrow = self.0.borrow_mut();
        let elvos_ref = &mut *elvos_borrow;

        f(elvos_ref)
    }
}

impl Os for ElvOs {
    // ElvOs is just a refcell around a ElvOsInner,
    // so all we have to do is borrow the inner.

    type SocketId = SocketHandle;

    fn socket(&self) -> Self::SocketId {
        self.0.borrow_mut().socket()
    }
    
    fn connect(&self, sock: Self::SocketId, local_endpoint: impl Into<IpListenEndpoint>, remote_endpoint: impl Into<IpEndpoint>) {
        self.0.borrow_mut().connect(sock, local_endpoint, remote_endpoint)
    }
    
    fn set_connect_callback(&self, sock: Self::SocketId, cb: impl FnMut() + 'static) {
        self.0.borrow_mut().set_connect_callback(sock, cb)
    }
    
    fn listen(&self, sock: Self::SocketId, local_endpoint: impl Into<IpListenEndpoint>) {
        self.0.borrow_mut().listen(sock, local_endpoint)
    }
    
    fn set_listen_callback(&self, sock: Self::SocketId, cb: impl FnMut() + 'static) {
        self.0.borrow_mut().set_listen_callback(sock, cb)
    }
    
    fn send(&self, sock: Self::SocketId, msg: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().send(sock, msg)
    }
    
    fn set_recv_callback(&self, sock: Self::SocketId, cb: impl FnMut() + 'static) {
        self.0.borrow_mut().set_recv_callback(sock, cb)
    }
    
    fn recv(&self, sock: Self::SocketId, msg: &mut [u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().recv(sock, msg)
    }
}

struct IncMachine<O: Os> {
    os: O,

}
