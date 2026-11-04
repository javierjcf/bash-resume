use std::io;
use std::path::Path;

use clap::Parser;
use stepwise::cli::Args;
use stepwise::log::Logger;
use stepwise::state::State;

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Demo provisional de T5: el flujo real (resolver rutas, preguntas…) llega en T8.
    let home = args.home.as_deref().unwrap_or(Path::new("demo"));
    let steps = args.steps.as_deref().unwrap_or(Path::new("demo/steps"));

    let mut log = Logger::open(&home.join("migration.log"))?;
    let state = State::load(&home.join("state.json"))?;
    log.info(&format!("Estado cargado de {}", home.display()));

    log.set_source(state.current_file.as_deref());
    if state.is_valid(steps) {
        log.info("El estado apunta a una línea válida");
    } else {
        log.warn("El estado no es válido: habrá que elegir en el menú");
    }
    log.set_source(None);
    log.error("Ejemplo de error (sin fuente)");
    Ok(())
}
