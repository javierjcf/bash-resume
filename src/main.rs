use std::process::ExitCode;

use bash_resume::cli::Args;
use clap::Parser;

fn main() -> ExitCode {
    match bash_resume::app::run(Args::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("❌ Error: {e}");
            ExitCode::FAILURE
        }
    }
}
