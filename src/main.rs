use core::str;

use simulator::run_sim_until;
use smoltcp::{
    iface::SocketHandle,
    wire::{EthernetAddress, IpAddress, IpCidr, IpEndpoint, Ipv4Address, Ipv4Cidr},
};
use tcp_machine::ElvOs;
use wire::Wire;

mod simulator;
mod tcp_machine;
mod wire;

const MILLISECOND: i64 = 1000;

/// Similar to println, but it also prints the file and line number.
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        println!("[{}:{}] {}", file!(), line!(), format!($($arg)*))
    }
}

fn main() {
    let time = 0;
    let end_time = 1000 * MILLISECOND;

    // let logger = env_logger::builder().target(env_logger::Target::Stdout).build();
    // log::set_boxed_logger(Box::new(logger)).expect("logger should not be set");
    env_logger::init();

    let mut node0 = ElvOs::new(time, 2, EthernetAddress([0, 0, 0, 0, 0, 0]));
    let mut node1 = ElvOs::new(time, 2, EthernetAddress([0, 0, 0, 0, 0, 1]));
    // 1 ms delay
    let mut wire = Wire::new(0, 1, MILLISECOND);

    // node 0 setup
    {
        node0.set_local_addrs(IpCidr::new(END0.addr, 24));
        let sock = node0.socket();
        node0.set_recv_callback(sock, ping_pong_callback);
        node0.set_connect_callback(sock, send_ping_callback);

        let cb_event = move |elvos: &mut ElvOs| {
            elvos.connect(sock, END0, END1);
        };
        node0.add_event(MILLISECOND * 45, cb_event);
    }

    // node 1 setup
    {
        node1.set_local_addrs(IpCidr::new(END1.addr, 24));
        let sock = node1.socket();
        node1.set_recv_callback(sock, ping_pong_callback);
        node1.listen(sock, END1);
    }

    run_sim_until(&mut [&mut node0, &mut node1, &mut wire], end_time);
}

fn ping_pong_callback(elvos: &mut ElvOs, handle: SocketHandle) {
    let msg = elvos.recv(handle);
    let msg_as_str = str::from_utf8(&msg);
    let out = match msg_as_str {
        Ok("ping") => "pong",
        Ok("pong") => "ping",
        other => panic!("Expected ping or pong but got {other:?}"),
    };
    log!("sending {out} to {}", elvos.receiver());
    let _ = elvos.send(handle, out.as_bytes());
}

fn send_ping_callback(elvos: &mut ElvOs, handle: SocketHandle) {
    elvos
        .send(handle, "ping".as_bytes())
        .expect("send should succeed");
}

const END0: IpEndpoint = IpEndpoint {
    addr: IpAddress::Ipv4(Ipv4Address([35, 0, 0, 1])),
    port: 60000,
};

const END1: IpEndpoint = IpEndpoint {
    addr: IpAddress::Ipv4(Ipv4Address([35, 0, 0, 2])),
    port: 60001,
};
