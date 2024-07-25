use std::collections::VecDeque;

use crate::simulator::{Index, Msg, Node, PollResult, Time};

/// Represents an outgoing message.s
/// Ordered so that the earliest events come first in Rust's BinaryHeap.
struct OutgoingMsg(Time, Index, Msg);

pub struct Wire {
    end1: Index,
    end2: Index,
    delay: Time,
    outgoing: VecDeque<OutgoingMsg>,
}

impl Wire {
    pub fn new(end1: Index, end2: Index, delay: Time) -> Wire {
        assert!(delay >= 0);
        Wire {
            end1,
            end2,
            delay,
            outgoing: VecDeque::new(),
        }
    }
}

impl Node for Wire {
    fn poll(&mut self, time: Time, incoming: PollResult) -> PollResult {
        for (sender, message) in incoming {
            let dest = if sender == self.end1 {
                self.end2
            } else if sender == self.end2 {
                self.end1
            } else {
                panic!("Tried to send to invalid machine")
            };

            let out_msg = OutgoingMsg(time + self.delay, dest, message);
            self.outgoing.push_back(out_msg);
        }

        // Send outgoing messages
        let mut result = Vec::new();
        while let Some(OutgoingMsg(out_time, _, _)) = self.outgoing.front() {
            if *out_time <= time {
                let OutgoingMsg(_, dest, msg) = self.outgoing.pop_front().unwrap();
                result.push((dest, msg));
            } else {
                break;
            }
        }
        result
    }

    fn poll_at(&mut self) -> Option<Time> {
        self.outgoing.front().map(|out| out.0)
    }
}
