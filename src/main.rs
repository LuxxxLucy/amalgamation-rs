use clap::Parser;
use std::path::PathBuf;

mod action;
mod interactive;
mod progress;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// GitHub repository URL
    url: String,

    /// Output file path
    #[arg(short, long)]
    output: PathBuf,

    /// Interactive mode
    #[arg(short, long)]
    interactive: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let action = action::AmalgamationAction {
        url: cli.url,
        output_pathname: cli.output,
        verbose: cli.verbose,
    };

    if cli.interactive {
        interactive::run_interactive_mode(action).await?;
    } else {
        action.execute().await?;
    }

    Ok(())
}
