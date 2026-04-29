use clap::{Parser, Subcommand};

mod ui;

#[derive(Debug, Parser)]
#[command(name = "ocotelolco")]
#[command(about = "Command line tools for active stock and ETF trading")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start the notes UI.
    Notes,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Notes => ui::notes::run(),
    }
}
