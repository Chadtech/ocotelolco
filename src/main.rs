use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ocotelolco")]
#[command(about = "Command line tools for active stock and ETF trading")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Ping,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Ping => {
            println!("First")
        }
    }
}
