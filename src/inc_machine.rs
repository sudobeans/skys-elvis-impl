use crate::machine::{Channel, Machine, Message, Time};

/// A machine that receives a number, increments it, then waits some time
/// before sending it back.
pub struct IncMachine {
    connections: Vec<(Channel, ConnectionInfo)>
}

#[derive(Debug, Clone, Copy)]
enum ConnectionInfo {
    /// Means it's going to send out a message
    WillSend(Time, u8),
    Waiting,
}

impl std::fmt::Display for IncMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dl = f.debug_list();
        for (chan, ci) in &self.connections {
            dl.entry(&(chan.receiver_name(), ci));
        };
        dl.finish()
    }
}

impl Machine for IncMachine {
    fn start() -> Self
    where
        Self: Sized {
        Self {
            connections: Vec::new()
        }
    }

    fn add_connection(&mut self, chan: Channel, time: Time) {
        // We say the channel with the earliest name in the alphabet
        // gets to send first. It's arbitrary.
        let ci = if chan.sender_name() < chan.receiver_name() {
            ConnectionInfo::WillSend(time + 1, 1)
        } else {
            ConnectionInfo::Waiting
        };
        self.connections.push((chan, ci));
    }

    fn poll(&mut self, time: Time) {
        for (chan, ci) in &mut self.connections {
            match (*ci, chan.recv()) {
                (ConnectionInfo::WillSend(when, num), _) => {
                    if  time >= when {
                        chan.send(Message::from_iter([num]))
                    }
                    *ci = ConnectionInfo::Waiting;
                }
                (ConnectionInfo::Waiting, Some(msg)) => {
                    if let Some(num) = msg.first() {
                        let new_num = num.saturating_add(1);
                        let new_time = time.checked_add(new_num as u64).expect("too much time passed");
                        *ci = ConnectionInfo::WillSend(new_time, new_num)
                    }
                }
                (ConnectionInfo::Waiting, None) => {}
            }
        }
    }

    fn poll_at(&self) -> Option<Time> {
        // Returns the nearest time in the future among all connections
        let mut min = None;
        for (_chan, ci) in self.connections.iter() {
            if let ConnectionInfo::WillSend(when, _num) = ci {
                match min {
                    Some(t) => min = Some(Time::min(*when, t)),
                    None => min = Some(*when),
                }
            }
        }
        min
    }
}