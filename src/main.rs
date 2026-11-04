use std::process::ExitCode;

use clap::Parser;
use stepwise::cli::Args;

fn main() -> ExitCode {
    match stepwise::app::run(Args::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("❌ Error: {e}");
            ExitCode::FAILURE
        }
    }
}
