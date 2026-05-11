use clap::{Parser, Subcommand};

mod transactions;
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
    /// Print a rerunnable Schwab transaction performance report.
    AnalyzeTransactions,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Notes => ui::run(),
        Command::AnalyzeTransactions => {
            if let Err(error) = transactions::print_cli_report() {
                eprintln!("failed to analyze transactions: {error}");
                std::process::exit(1);
            }
        }
    }
}
