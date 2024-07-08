use std::{collections::{BTreeMap, BTreeSet, BinaryHeap}, time::{Duration, Instant}};

use inc_machine::IncMachine;
use machine::{Channel, Machine, Time};

mod machine;
mod inc_machine;

const HELP_MSG: &str = "\
##############################################################################
This simulation is interactive.

Commands:
- help :                prints this message.
- add :                 adds a machine.
- connect idx1 idx2 :   connects the machine at index idx1 to the machine at idx2.
- step :                moves forward in time to when the next thing happens.
- realtime secs :       runs the simulation for secs real-time seconds.
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
    Quit,
}

/// stupid error handling
struct StupidError(String);

impl<T: std::error::Error> From<T> for StupidError {
    fn from(value: T) -> Self {
        StupidError(value.to_string())
    }
}

fn get_command() -> Result<Command, StupidError> {
    use Command::*;

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
        } ,
        ["step"] => Ok(Step),
        ["realtime", secs] => {
            let secs = secs.parse()?;
            Ok(Realtime(Duration::from_secs(secs)))
        }
        ["quit"] => Ok(Quit),
        _ => Err(StupidError("invalid command, type \"help\" to see a list of valid commands".to_string()))?
    }
}


fn main() {
    // each entry represents the time a machine should be polled 
    // and its index in the machines list
    let mut events: BTreeSet<(Time, Index)> = BTreeSet::new();
    let mut machines: Vec<Box<dyn Machine>> = Vec::new();


    // The simulation is terminal-interactive.
    // in the real sim we would probably use the NDL or something for this.
    println!("{HELP_MSG}");

    loop {
        match get_command() {
            Ok(comm) => match comm {
                Command::Help => println!("{HELP_MSG}"),
                Command::Add => {
                    let machine = IncMachine::start();
                    let time = machine.poll_at();

                    machines.push(Box::new(machine));

                    if let Some(t) = time {
                        events.insert((t, machines.len()-1));
                    }
                },
                Command::Connect(name1, name2) => {
                    let (end1, end2, update) = Channel::new(name1, name2);


                },
                Command::Step => todo!(),
                Command::Realtime(_) => todo!(),
                Command::Quit => todo!(),
            }
            Err(e) => println!("Error reading your input: {e}"),
        }
    }
}
