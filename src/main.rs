use stepwise::parser::Line;

fn main() {
    // Demo provisional de T1: se sustituirá en T2.
    let samples = [
        "docker compose up -d",
        "#> EMPEZANDO SETUP",
        "#. Parada manual",
        "# comentario",
        "   ",
    ];
    for line in samples {
        println!("{:?}", Line::new(line));
    }
}
