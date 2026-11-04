use std::io;
use std::path::Path;

use clap::Parser;
use stepwise::cli::Args;
use stepwise::log::Logger;
use stepwise::runner::{Mode, run_steps};
use stepwise::state::State;

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Demo provisional de T7: ejecuta desde el estado guardado, sin preguntar. Las preguntas llegan en T8.
    let home = args.home.as_deref().unwrap_or(Path::new("demo"));
    let steps = args.steps.as_deref().unwrap_or(Path::new("demo/steps"));
    let workdir = match args.workdir {
        Some(dir) => dir,
        None => std::env::current_dir()?,
    };
    let mode = match std::env::var("STEPWISE_ONCE") {
        Ok(_) => Mode::Once,
        Err(_) => Mode::All,
    };

    let mut log = Logger::open(&home.join("migration.log"))?;
    let mut state = State::load(&home.join("state.json"))?;
    let onerror = home.join("onerror");

    let outcome = run_steps(&mut state, steps, &workdir, &onerror, &mut log, mode)?;
    println!("\n> Resultado: {outcome:?}");
    print!("{state}");
    Ok(())
}
