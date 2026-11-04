use std::io;
use std::path::Path;

use clap::Parser;
use stepwise::cli::Args;
use stepwise::parser::{list_step_files, parse_file};

fn main() -> io::Result<()> {
    let args = Args::parse();
    println!("{args:#?}");

    // Demo provisional de T3: el flujo real (estado, preguntas…) llega en T8.
    let dir = args.steps.as_deref().unwrap_or(Path::new("demo/steps"));
    for file in list_step_files(dir)? {
        println!("== {file}");
        for (i, line) in parse_file(&dir.join(&file))?.iter().enumerate() {
            println!(
                "[{:03}] {:<7} {}",
                i + 1,
                format!("{:?}", line.kind),
                line.text
            );
        }
    }
    Ok(())
}
