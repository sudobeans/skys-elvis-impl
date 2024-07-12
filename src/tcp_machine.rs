use smoltcp::{
    iface::SocketSet, phy::Device, wire::{IpEndpoint, IpListenEndpoint}
};

use crate::util::Channel;

use std::future::Future;

trait Os {
    type Socket: std::io::Read + std::io::Write;

    fn add_connection(chan: Channel);

    async fn connect(
        &mut self,
        local_endpoint: impl Into<IpEndpoint>,
        remote_endpoint: impl Into<IpListenEndpoint>,
    ) -> Self::Socket;

    async fn listen(&mut self, local_endpoint: impl Into<IpListenEndpoint>) -> Self::Socket;
}

struct ElvOs {
    channels: Vec<Channel>,
    sockets: SocketSet<'static>,
}

struct ElvOsSocket {
    channels: Vec<Channel>,
}

impl Os for ElvOs {
    type Socket = ElvOsSocket;

    fn add_connection(chan: Channel) {}

    async fn connect(
        &mut self,
        local_endpoint: impl Into<IpEndpoint>,
        remote_endpoint: impl Into<IpListenEndpoint>,
    ) -> Self::Socket {
    }

    async fn listen(&mut self, local_endpoint: impl Into<IpListenEndpoint>) -> Self::Socket {}
}

impl Device for ElvOs {}
