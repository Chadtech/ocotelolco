use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod palette;
mod transactions;
mod ui;
mod website;

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
    /// Print Schwab transaction performance grouped by ticker tags.
    AnalyzeTickerTags {
        /// Print only tag names and returns.
        #[arg(long)]
        returns_only: bool,
    },
    /// Generate a static trading success website HTML file.
    MakeWebsite {
        /// Path to write the generated HTML file. Defaults to outputs/ocotelolco.html.
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
    },
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
        Command::AnalyzeTickerTags { returns_only } => {
            if let Err(error) = transactions::print_tag_cli_report(returns_only) {
                eprintln!("failed to analyze ticker tags: {error}");
                std::process::exit(1);
            }
        }
        Command::MakeWebsite { output } => {
            let output = output.unwrap_or_else(website::default_output_path);
            if let Err(error) = website::write_site(&output) {
                eprintln!("failed to make website: {error}");
                std::process::exit(1);
            }
            println!("Wrote website to {}", output.display());
        }
    }
}
