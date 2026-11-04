use std::io;
use std::path::Path;

use clap::Parser;
use stepwise::cli::Args;
use stepwise::state::State;

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Demo provisional de T4: el flujo real (resolver rutas, preguntas…) llega en T8.
    let home = args.home.as_deref().unwrap_or(Path::new("demo"));
    let steps = args.steps.as_deref().unwrap_or(Path::new("demo/steps"));

    let state = State::load(&home.join("state.json"))?;
    println!("> Estado en {}", home.display());
    print!("{state}");
    println!(
        "¿Válido respecto a {}? {}",
        steps.display(),
        state.is_valid(steps)
    );
    Ok(())
}
