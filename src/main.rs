use simulator::run_sim_until;
use smoltcp::wire::{EthernetAddress, HardwareAddress};
use tcp_machine::ElvOs;
use wire::Wire;

mod tcp_machine;
mod simulator;
mod wire;

const MILLISECOND: i64 = 100000;

/// Similar to println, but it also prints the file and line number.
macro_rules! log {
    ($($arg:tt)*) => { println!("[{}:{}] {}", file!(), line!(), format!($($arg)*)) }
}

fn main() {
    let time = 0;
    let end_time = 100 * MILLISECOND;

    let mut node0 = ElvOs::new(time, 2, EthernetAddress([0, 0, 0, 0, 0, 0]));
    let mut node1 = ElvOs::new(time, 2, EthernetAddress([0, 0, 0, 0, 0, 1]));
    // 1 ms delay
    let mut wire = Wire::new(0, 1, MILLISECOND);

    run_sim_until(&mut [&mut node0, &mut node1, &mut wire], end_time);
}
