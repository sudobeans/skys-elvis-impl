use crate::{
    util::{Channel, Machine, Time},
    Index,
};

/// A machine that receives a number, increments it, then waits some time
/// before sending it back.
pub struct IncMachine {
    connections: Vec<(Channel, ConnectionInfo)>,
}

#[derive(Debug, Clone, Copy)]
enum ConnectionInfo {
    /// Means it's going to send out a message
    WillSend(Time, u8),
    Waiting,
}

impl std::fmt::Debug for IncMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dl = f.debug_list();
        for (chan, ci) in &self.connections {
            dl.entry(&(chan.other_index(), ci));
        }
        dl.finish()
    }
}

impl Machine for IncMachine {
    fn start(_index: Index) -> Self
    where
        Self: Sized,
    {
        Self {
            connections: Vec::new(),
        }
    }

    fn add_connection(&mut self, chan: Channel, time: Time) {
        // We say the channel with the lowest index gets to send first.
        // it's arbitrary
        let ci = if chan.index() < chan.other_index() {
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
                    if time >= when {
                        chan.send(&[num])
                    }
                    *ci = ConnectionInfo::Waiting;
                }
                (ConnectionInfo::Waiting, msgs) => {
                    if let Some(num) = msgs.first() {
                        let new_num = num.saturating_add(1);
                        let new_time = time
                            .checked_add(new_num as u64)
                            .expect("too much time passed");
                        *ci = ConnectionInfo::WillSend(new_time, new_num)
                    }
                }
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
