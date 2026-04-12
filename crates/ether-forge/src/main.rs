use clap::{Parser, Subcommand};

// FIXME(T6): drop `allow(dead_code)` once a subcommand wires Task::load_all in.
#[allow(dead_code)]
mod task;

/// Ether development process CLI — backlog management and workflow automation.
#[derive(Parser, Debug)]
#[command(name = "ether-forge", about, version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => {
            println!("ether-forge: no subcommand given. See --help.");
            Ok(())
        }
    }
}
