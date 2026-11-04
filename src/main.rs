use std::io;
use std::path::Path;

use clap::Parser;
use stepwise::cli::Args;
use stepwise::exec::execute;
use stepwise::log::Logger;
use stepwise::parser::{Kind, parse_file};

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Demo provisional de T6: ejecuta los comandos de un fichero. El runner real llega en T7.
    let home = args.home.as_deref().unwrap_or(Path::new("demo"));
    let steps = args.steps.as_deref().unwrap_or(Path::new("demo/steps"));
    let workdir = match args.workdir {
        Some(dir) => dir,
        None => std::env::current_dir()?,
    };

    let mut log = Logger::open(&home.join("migration.log"))?;
    log.set_source(Some("01-preparar"));

    for line in parse_file(&steps.join("01-preparar"))? {
        if line.kind == Kind::Cmd && !execute(&line.text, &workdir, &log) {
            log.error("Parada: ha fallado un comando");
            break;
        }
    }
    Ok(())
}
