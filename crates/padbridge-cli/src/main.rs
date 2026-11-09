use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about = "Control the padbridge daemon")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status,
    Shutdown,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Status => println!("padbridge daemon status is not implemented yet"),
        Command::Shutdown => println!("padbridge daemon shutdown is not implemented yet"),
    }
}