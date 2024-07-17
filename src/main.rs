use std::{collections::BTreeSet, io::Write, time::Duration};

use inc_machine::IncMachine;
use util::{take_all, Channel, Machine, Time};

mod inc_machine;
mod tcp_machine;
mod util;
mod os_traits;

const HELP_MSG: &str = "\
##############################################################################
This simulation is interactive.

Commands:
- help :                prints this message.
- add :                 adds a machine.
- connect idx1 idx2 :   connects the machine at index idx1 to the machine at idx2.
- step :                moves forward in time to when the next thing happens.
- realtime secs :       runs the simulation for secs real-time seconds.
- print :               prints machine information.
- log :                 toggles logging machine information after each step.
- quit :                quits the simulation.

Currently, the machines in the sim start by sending 1.
Whenever they receive a number, they increment it and send it back.
##############################################################################
";

type Index = usize;

enum Command {
    Help,
    Add,
    Connect(Index, Index),
    Step,
    Realtime(Duration),
    Print,
    Log,
    Quit,
}

/// stupid error handling
struct StupidError(String);

impl<T: std::error::Error> From<T> for StupidError {
    fn from(value: T) -> Self {
        StupidError(value.to_string())
    }
}

impl std::fmt::Debug for StupidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn get_command() -> Result<Command, StupidError> {
    use Command::*;

    print!(">>> ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;

    let tokens = Vec::from_iter(line.split_whitespace().map(str::to_lowercase));
    let refs = Vec::from_iter(tokens.iter().map(String::as_str));
    match &*refs {
        ["help"] => Ok(Help),
        ["add"] => Ok(Add),
        ["connect", str1, str2] => {
            let idx1 = str1.parse()?;
            let idx2 = str2.parse()?;
            Ok(Connect(idx1, idx2))
        }
        ["step"] => Ok(Step),
        ["realtime", secs] => {
            let secs = secs.parse()?;
            Ok(Realtime(Duration::from_secs(secs)))
        }
        ["print"] => Ok(Print),
        ["log"] => Ok(Log),
        ["quit"] => Ok(Quit),
        _ => Err(StupidError(
            "invalid command, type \"help\" to see a list of valid commands".to_string(),
        ))?,
    }
}

/// moves messages from the send channel to the recv channel.
/// returns true if a message was moved.
fn move_messages(send: &Channel, recv: &Channel) -> bool {
    let b = take_all(send.outbox.borrow_mut().as_mut());
    if b.is_empty() {
        false
    } else {
        recv.inbox.borrow_mut().extend_from_slice(&b);
        true
    }
}

fn main() {
    // each entry represents the time a machine should be polled
    // and its index in the machines list
    let mut events: BTreeSet<(Time, Index)> = BTreeSet::new();
    let mut last_time: Time = 0;
    let mut machines: Vec<Box<dyn Machine>> = Vec::new();
    // The channels used by the simulation.
    // 1st channel is the one that was given to machine 1,
    // 2nd channel is the one that was given to machine 2.
    // Note: machine 1 sends a message to sim, then sim passes on message
    // to machine 2. (Sim is the man in the middle.)
    let mut channels: Vec<(Channel, Channel)> = Vec::new();

    println!("{HELP_MSG}");

    // Runs the next event in time.
    // this function kinda sucks
    fn time_step(
        events: &mut BTreeSet<(Time, Index)>,
        last_time: &mut Time,
        machines: &mut Vec<Box<dyn Machine>>,
        channels: &mut Vec<(Channel, Channel)>,
    ) {
        if let Some((time, index)) = events.pop_first() {
            *last_time = time;
            machines[index].poll(time);

            for (chan1, chan2) in channels {
                if move_messages(chan1, chan2) {
                    events.insert((time, chan1.other_index()));
                }
                if move_messages(chan2, chan1) {
                    events.insert((time, chan2.other_index()));
                }
            }

            if let Some(next_time) = machines[index].poll_at() {
                events.insert((next_time, index));
            }
        }
    }

    fn print_state(time: Time, machines: &Vec<Box<dyn Machine>>, events: &BTreeSet<(u64, usize)>) {
        println!("Time: {time}");
        println!("Events: {events:?}");
        println!("machines:");
        for (index, machine) in machines.iter().enumerate() {
            println!("{index}: {machine:?}");
        }
        println!();
    }

    let mut should_log = true;
    loop {
        match get_command() {
            Ok(comm) => match comm {
                Command::Help => println!("{HELP_MSG}"),
                Command::Add => {
                    let index = machines.len();
                    let machine = IncMachine::start(index);
                    let poll_time = machine.poll_at();

                    machines.push(Box::new(machine));

                    if let Some(t) = poll_time {
                        events.insert((t, machines.len() - 1));
                    }
                }
                Command::Connect(name1, name2) => {
                    let (end1, end2) = Channel::new(name1, name2);

                    machines[name1].add_connection(end1.clone(), last_time);
                    machines[name2].add_connection(end2.clone(), last_time);

                    channels.push((end1, end2));

                    if let Some(time) = machines[name1].poll_at() {
                        events.insert((time, name1));
                    }
                    if let Some(time) = machines[name2].poll_at() {
                        events.insert((time, name2));
                    }
                }
                Command::Step => {
                    if events.is_empty() {
                        println!("Event queue is empty!");
                    } else {
                        time_step(&mut events, &mut last_time, &mut machines, &mut channels);
                    }
                }
                Command::Realtime(secs) => {
                    // note that this will get out of sync with real time
                    // if polling takes too long.
                    // This is kept simple for demo purposes.
                    let virt_end = last_time + secs.as_secs();

                    loop {
                        let next_time = match events.first().cloned() {
                            Some((next_time, _index)) => {
                                if next_time > virt_end {
                                    break;
                                }
                                next_time
                            }
                            None => break,
                        };

                        let wait_time = Duration::from_secs(next_time - last_time);
                        std::thread::sleep(wait_time);
                        time_step(&mut events, &mut last_time, &mut machines, &mut channels);

                        if should_log {
                            print_state(last_time, &machines, &events);
                        }
                    }
                }
                Command::Print => print_state(last_time, &machines, &events),
                Command::Log => should_log = !should_log,
                Command::Quit => break,
            },
            Err(e) => println!("Error reading your input: {e:?}"),
        }
        if should_log {
            print_state(last_time, &machines, &events);
        }
    }
}
