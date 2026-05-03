use winit::event_loop::ActiveEventLoop;

use crate::App;

#[derive(Debug)]
pub enum CliCommand {
    Quit
}

pub fn parse_command(input: &str) -> Option<CliCommand> {
    let parts: Vec<&str> = input.trim().split_whitespace().collect();

    match parts.as_slice() {
        ["quit"] | ["exit"] => Some(CliCommand::Quit),
        _ => None,
    }
}

pub fn execute_cli_commands(app: &mut App, event_loop: &ActiveEventLoop, cmd: CliCommand){
    match cmd {
        CliCommand::Quit => event_loop.exit(),
    }
}