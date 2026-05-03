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