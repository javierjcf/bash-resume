use std::io;
use std::path::Path;

use stepwise::parser::{list_step_files, parse_file};

fn main() -> io::Result<()> {
    // Demo provisional de T2: la ruta se recibirá con clap en T3.
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "demo/steps".to_string());
    let dir = Path::new(&dir);

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
