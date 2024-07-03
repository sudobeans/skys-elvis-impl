use std::{collections::{BTreeMap, BTreeSet, BinaryHeap}, time::{Duration, Instant}};

use inc_machine::IncMachine;
use machine::{Machine, Time};

mod machine;
mod inc_machine;

const HELP_MSG: &str = "\
##############################################################################
This simulation is interactive.

Commands:
- help :                prints this message.
- add name :            adds a machine with the given name.
- connect name1 name2 : connects the machine name1 to the machine name2.
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
    Add(String),
    Connect(String, String),
    Step,
    Realtime(Duration),
    Quit,
}

fn get_command() -> Result<Command, String> {
    use Command::*;

    let mut line = String::new();
    if let Err(e) = std::io::stdin().read_line(&mut line) {
        return Err(e.to_string());
    }

    let tokens = Vec::from_iter(line.split_whitespace().map(str::to_lowercase));
    let refs = Vec::from_iter(tokens.iter().map(String::as_str));
    match &*refs {
        ["help"] => Ok(Help),
        ["add", name] => Ok(Add(name.to_string())),
        ["connect", name1, name2] => Ok(Connect(name1.to_string(), name2.to_string())),
        ["step"] => Ok(Step),
        ["realtime", secs] => {
            match u64::from_str_radix(secs, 10) {
                Ok(dur) => Ok(Realtime(Duration::from_secs(dur))),
                Err(e) => Err(e.to_string()),
            }
        }
        ["quit"] => Ok(Quit),
        _ => Err("invalid command, type \"help\" to see a list of valid commands".to_string())
    }
}


fn main() {
    // each entry represents the time a machine should be polled 
    // and its index in the machines list
    let mut events: BTreeSet<(Time, usize)> = BTreeSet::new();
    let mut machines: Vec<Box<dyn Machine>> = Vec::new();

    // The simulation is terminal-interactive.
    // in the real sim we would probably use the NDL or something for this.
    println!("{HELP_MSG}");

    loop {
        match get_command() {
            Ok(comm) => match comm {
                Command::Help => println!("{HELP_MSG}"),
                Command::Add(name) => {
                    let machine = IncMachine::start(name);
                    let time = machine.poll_at();

                    machines.push(Box::new(machine));

                    if let Some(t) = time {
                        events.insert((t, machines.len()-1));
                    }
                },
                Command::Connect(name1, name2) => {
                    
                },
                Command::Step => todo!(),
                Command::Realtime(_) => todo!(),
                Command::Quit => todo!(),
            }
            Err(e) => println!("Error reading your input: {e}"),
        }
    }
}
