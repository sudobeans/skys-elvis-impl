use core::str;
use std::fmt::Write;

use crate::log;

pub type Msg = Vec<u8>;

pub type Index = usize;

pub type Time = i64;

pub type PollResult = Vec<(Index, Msg)>;

/// A node is a thing in a simulation that is separated from other nodes.
/// It could represent a machine on a network, or anything at all.
///
/// Using this trait can be a little awkward, since it requires your
/// entire node to be a state machine. For ease of use, consider using the
/// [`SchedulerNode`] trait.
pub trait Node {
    /// Tells the node the current time, and messages it has received,
    /// so it can act accordingly.
    ///
    /// # Parameters
    ///
    /// * `time` - the time that the node is being polled
    ///
    /// * `incoming` - the incoming messages. Each pair contains the
    /// index of the machine that sent it, and the message itself.
    /// May be empty.
    ///
    /// # Returns
    /// The node returns a vec of messages it wants to send out, along with
    /// the indices of each node that will receive the message.
    ///
    /// # Panics
    ///
    /// Nodes are allowed to panic if the time passed in
    /// is less than the last one passed to `poll` or `receive`.
    /// (Basically, nodes can't go back in time.)
    fn poll(&mut self, time: Time, incoming: PollResult) -> PollResult;

    /// Returns the next time this machine should be polled.
    fn poll_at(&mut self) -> Option<Time>;
}

/// Runs a simulation of the machines until the given time has passed.
pub fn run_sim_until(nodes: &mut [&mut dyn Node], end_time: Time) {
    // The current time.
    let mut time = match earliest_poll_time(nodes) {
        Some((_index, time)) => time,
        None => return,
    };

    // the messages each machine needs to receive
    let mut mailboxes: Vec<PollResult> = vec![PollResult::new(); nodes.len()];

    while let Some((i, t)) = machine_to_poll(nodes, &mailboxes, time) {
        time = t;
        log!("{i} polled at {time}");
        if time > end_time {
            break;
        }

        let outgoing = nodes[i].poll(time, take_all(&mut mailboxes[i]));

        // prints out the packets sent
        for (dest, msg) in &outgoing {
            log!("packet from {i} to {dest}: {}", packet_to_str(msg).unwrap());
        }

        // deliver messages to mailboxes
        for (destination, msg) in outgoing {
            mailboxes[destination].push((i, msg));
        }
    }
}

fn machine_to_poll(
    nodes: &mut [&mut dyn Node],
    mailboxes: &[PollResult],
    current_time: Time,
) -> Option<(Index, Time)> {
    for node in nodes.iter_mut() {
        if let Some(time) = node.poll_at() {
            assert!(
                time >= current_time,
                "machines should not be polled in the past"
            );
        }
    }

    // if a machine has messages in its mailbox, it should be polled first
    #[allow(clippy::needless_range_loop)]
    for i in 0..mailboxes.len() {
        if !mailboxes[i].is_empty() {
            return Some((i, current_time));
        }
    }

    earliest_poll_time(nodes)
}

/// Goes through the nodes and returns the index of the
/// one with the earilest poll time.
pub fn earliest_poll_time(nodes: &mut [&mut dyn Node]) -> Option<(Index, Time)> {
    let mut earliest: Option<(Index, Time)> = None;

    #[allow(clippy::needless_range_loop)]
    for i in 0..nodes.len() {
        let i_poll_at = nodes[i].poll_at();
        match (i_poll_at, earliest) {
            // both the earliest machine and the current machine
            // have a set poll time
            (Some(i_time), Some((_index, early_time))) => {
                if i_time < early_time {
                    earliest = Some((i, i_time));
                }
            }
            // the earliest time is not yet set
            (Some(i_time), None) => {
                earliest = Some((i, i_time));
            }
            _ => (),
        }
    }
    earliest
}

/// Removes all values in v, and puts them in the result.
fn take_all(v: &mut PollResult) -> PollResult {
    let mut result = Vec::new();
    std::mem::swap(&mut result, v);
    result
}

/// Interprets a bunch of bytes as an ethernet-ip-tcp packet
/// and turns them into a string
fn packet_to_str(packet: &[u8]) -> Result<String, smoltcp::wire::Error> {
    use smoltcp::wire::*;

    let mut result = String::new();
    let eth = EthernetFrame::new_checked(packet)?;
    let _ = writeln!(result, "{eth}");
    if eth.ethertype() == smoltcp::wire::EthernetProtocol::Ipv4 {
        let ip = Ipv4Packet::new_checked(eth.payload())?;
        let _ = writeln!(result, "\t{ip}");
        if ip.next_header() == smoltcp::wire::IpProtocol::Tcp {
            let tcp = TcpPacket::new_checked(ip.payload())?;
            let _ = writeln!(result, "\t{tcp}");

            let payload = str::from_utf8(tcp.payload());
            let _ = writeln!(result, "\tPayload: {payload:?}");
        }
    }

    Ok(result)
}
