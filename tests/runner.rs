//! Tests de integración del recorrido de pasos: usan la librería como lo hará el binario.
//!
//! Los comandos escriben en el fichero `out` del workdir, así se comprueba qué se ejecutó y en qué orden.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use bash_resume::engine::runner::{Mode, Outcome, run_steps};
use bash_resume::engine::state::State;
use bash_resume::engine::timings::Timings;
use bash_resume::log::Logger;

/// Proyecto de prueba en un directorio temporal: `steps/`, `state.json`, `migration.log` y `onerror`.
struct Project {
    dir: PathBuf,
}

impl Project {
    /// Crea el proyecto con los ficheros de pasos `(nombre, contenido)`.
    fn new(name: &str, steps: &[(&str, &str)]) -> io::Result<Self> {
        let dir =
            std::env::temp_dir().join(format!("bash-resume-it-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("steps"))?;
        for (file, content) in steps {
            fs::write(dir.join("steps").join(file), content)?;
        }
        Ok(Self { dir })
    }

    fn state(&self) -> io::Result<State> {
        State::load(&self.dir.join("state.json"))
    }

    /// Ejecuta los pasos con el estado guardado y devuelve el resultado.
    fn run(&self, mode: Mode) -> io::Result<Outcome> {
        let mut state = self.state()?;
        let mut log = Logger::open(&self.dir.join("migration.log"))?;
        let mut timings = Timings::load(&self.dir.join("times.json"));
        run_steps(
            &mut state,
            &self.dir.join("steps"),
            &self.dir,
            &self.dir.join("onerror"),
            &mut log,
            mode,
            &mut timings,
        )
    }

    /// Contenido de un fichero del proyecto (vacío si no existe).
    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.dir.join(name)).unwrap_or_default()
    }

    /// Líneas escritas en `out` por los comandos.
    fn out(&self) -> Vec<String> {
        let content = fs::read_to_string(self.dir.join("out")).unwrap_or_default();
        content.lines().map(str::to_string).collect()
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Project {
    /// Borra el directorio temporal al terminar el test, aunque falle.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Comprueba la posición guardada en el estado.
fn assert_position(state: &State, file: &str, line: usize, command: &str) {
    assert_eq!(state.current_file.as_deref(), Some(file));
    assert_eq!(state.last_line_index, Some(line));
    assert_eq!(state.last_command.as_deref(), Some(command));
}

#[test]
fn runs_all_commands_in_order_across_files() -> io::Result<()> {
    let p = Project::new(
        "all",
        &[
            ("02-b", "echo c >> out\n"),
            (
                "01-a",
                "#> hola\necho a >> out\n# comentario\n\necho b >> out\n",
            ),
            (".oculto", "echo nunca >> out\n"),
        ],
    )?;

    assert_eq!(p.run(Mode::All)?, Outcome::Finished);

    assert_eq!(p.out(), ["a", "b", "c"]);
    // Al terminar, el estado se queda en el último comando (como en Python).
    assert_position(&p.state()?, "02-b", 1, "echo c >> out");
    Ok(())
}

#[test]
fn cd_does_not_persist_between_lines() -> io::Result<()> {
    let p = Project::new(
        "cd",
        &[("01-a", "mkdir sub && cd sub && pwd >> ../out\npwd >> out\n")],
    )?;

    p.run(Mode::All)?;

    let out = p.out();
    assert!(out[0].ends_with("/sub"), "{out:?}");
    assert!(
        !out[1].ends_with("/sub"),
        "la segunda línea vuelve al workdir: {out:?}"
    );
    Ok(())
}

#[test]
fn failure_saves_position_and_runs_onerror() -> io::Result<()> {
    let p = Project::new("fail", &[("01-a", "echo a >> out\nfalse\necho b >> out\n")])?;
    fs::write(p.path("onerror"), "# aviso\necho onerror >> out\n")?;

    assert_eq!(p.run(Mode::All)?, Outcome::Failed);

    assert_eq!(p.out(), ["a", "onerror"]);
    assert_position(&p.state()?, "01-a", 2, "false");
    Ok(())
}

#[test]
fn failure_without_onerror_file_still_fails() -> io::Result<()> {
    let p = Project::new("fail-noerr", &[("01-a", "false\n")])?;
    assert_eq!(p.run(Mode::All)?, Outcome::Failed);
    let log = fs::read_to_string(p.path("migration.log"))?;
    assert!(log.contains("No se encontró fichero 'onerror'"), "{log}");
    Ok(())
}

#[test]
fn resume_reruns_failed_command() -> io::Result<()> {
    let p = Project::new(
        "resume-fail",
        &[("01-a", "echo a >> out\ntest -f ok\necho b >> out\n")],
    )?;
    assert_eq!(p.run(Mode::All)?, Outcome::Failed);

    fs::write(p.path("ok"), "")?; // "arreglamos" el problema y reanudamos
    assert_eq!(p.run(Mode::All)?, Outcome::Finished);

    assert_eq!(p.out(), ["a", "b"], "`a` no se repite");
    Ok(())
}

#[test]
fn stop_line_halts_and_resume_stops_again() -> io::Result<()> {
    let p = Project::new(
        "stop",
        &[("01-a", "echo a >> out\n#. revisar\necho b >> out\n")],
    )?;

    assert_eq!(p.run(Mode::All)?, Outcome::Stopped);
    assert_position(&p.state()?, "01-a", 2, "#. revisar");

    // Como en Python: "Continuar" vuelve a parar en el mismo `#.`.
    assert_eq!(p.run(Mode::All)?, Outcome::Stopped);
    assert_eq!(p.out(), ["a"]);
    Ok(())
}

#[test]
fn once_runs_a_single_command_and_points_to_the_next() -> io::Result<()> {
    let p = Project::new(
        "once",
        &[
            ("01-a", "#> x\necho a >> out\n"),
            ("02-b", "\necho b >> out\n"),
        ],
    )?;

    assert_eq!(p.run(Mode::Once)?, Outcome::OnceDone);
    assert_eq!(p.out(), ["a"]);
    assert_position(&p.state()?, "02-b", 2, "echo b >> out");

    assert_eq!(p.run(Mode::Once)?, Outcome::OnceDone);
    assert_eq!(p.out(), ["a", "b"]);
    Ok(())
}

#[test]
fn starts_from_saved_position() -> io::Result<()> {
    let p = Project::new(
        "start",
        &[
            ("01-a", "echo a >> out\necho b >> out\n"),
            ("02-b", "echo c >> out\n"),
        ],
    )?;
    let mut state = p.state()?;
    state.set_position("01-a", "echo b >> out", 2)?;

    p.run(Mode::All)?;

    assert_eq!(p.out(), ["b", "c"]);
    Ok(())
}

#[test]
fn unknown_saved_file_starts_from_the_beginning() -> io::Result<()> {
    let p = Project::new("unknown", &[("01-a", "echo a >> out\n")])?;
    p.state()?.set_position("99-borrado", "echo x", 5)?;

    p.run(Mode::All)?;

    assert_eq!(p.out(), ["a"]);
    Ok(())
}

#[test]
fn empty_steps_dir_finishes_without_doing_anything() -> io::Result<()> {
    let p = Project::new("empty", &[])?;
    assert_eq!(p.run(Mode::All)?, Outcome::Finished);
    assert!(!Path::new(&p.path("out")).exists());
    Ok(())
}

// ---- Tiempos (spec §6.1) ----

#[test]
fn successful_commands_are_timed_and_summarized() -> io::Result<()> {
    let p = Project::new(
        "times-ok",
        &[
            ("01-a", "echo a >> out\n#> nota\necho b >> out\n"),
            ("02-b", "echo c >> out\n"),
        ],
    )?;
    assert_eq!(p.run(Mode::All)?, Outcome::Finished);

    let times = p.read("times.json");
    for cmd in ["echo a >> out", "echo b >> out", "echo c >> out"] {
        assert!(times.contains(&format!("\"{cmd}\": 0")), "{times}");
    }
    let log = p.read("migration.log");
    assert!(
        log.contains("▶ [2/3] Ejecutando comando: echo b >> out"),
        "{log}"
    );
    assert!(log.contains("⏱ Fichero 01-a: 00:00:00"), "{log}");
    assert!(log.contains("⏱ Resumen de tiempos:"), "{log}");
    assert!(log.contains("   Total   00:00:00"), "{log}");
    assert!(log.contains("⏱ Esta ejecución: 00:00:00"), "{log}");
    Ok(())
}

#[test]
fn estimate_uses_saved_times_and_counts_missing_ones() -> io::Result<()> {
    let p = Project::new(
        "times-eta",
        &[("01-a", "echo a >> out\necho b >> out\necho c >> out\n")],
    )?;
    fs::write(
        p.path("times.json"),
        r#"{ "echo a >> out": 120, "echo b >> out": 60 }"#,
    )?;
    assert_eq!(p.run(Mode::All)?, Outcome::Finished);

    let log = p.read("migration.log");
    assert!(
        log.contains(
            "▶ [1/3] Ejecutando comando: echo a >> out   (quedan ~00:03:00, 1 sin tiempo)"
        ),
        "{log}"
    );
    assert!(
        log.contains("▶ [3/3] Ejecutando comando: echo c >> out\n"),
        "sin datos, sin paréntesis:\n{log}"
    );
    // Solo se añade texto a las líneas que ya había: ninguna línea extra por comando.
    let per_command = log.lines().filter(|l| l.contains("[01-a]")).count();
    assert_eq!(
        per_command,
        3 * 4 + 1,
        "Ejecutando + 2 de resultado + Tiempo por comando, y el total del fichero:\n{log}"
    );
    Ok(())
}

#[test]
fn failed_command_keeps_its_previous_time() -> io::Result<()> {
    let p = Project::new("times-fail", &[("01-a", "false\n")])?;
    fs::write(p.path("times.json"), r#"{ "false": 99 }"#)?;
    assert_eq!(p.run(Mode::All)?, Outcome::Failed);

    assert!(p.read("times.json").contains("\"false\": 99"));
    let log = p.read("migration.log");
    assert!(log.contains("⏱ Esta ejecución"), "{log}");
    assert!(
        !log.contains("Resumen de tiempos"),
        "solo al terminar todo:\n{log}"
    );
    Ok(())
}

#[test]
fn summary_after_resume_includes_earlier_commands() -> io::Result<()> {
    let p = Project::new(
        "times-resume",
        &[("01-a", "echo a >> out\ntest -f ok\necho b >> out\n")],
    )?;
    assert_eq!(p.run(Mode::All)?, Outcome::Failed);
    fs::write(p.path("ok"), "")?;
    assert_eq!(p.run(Mode::All)?, Outcome::Finished);

    assert_eq!(p.out(), ["a", "b"], "`a` no se repite");
    assert!(
        p.read("times.json").contains("\"echo a >> out\""),
        "el tiempo de `a` se guardó antes del fallo"
    );
    let log = p.read("migration.log");
    assert!(
        !log.contains("sin tiempo"),
        "el resumen cuenta `a` aunque no se repitió:\n{log}"
    );
    Ok(())
}

#[test]
fn command_output_never_goes_to_the_log() -> io::Result<()> {
    let p = Project::new("times-output", &[("01-a", "echo SAL\"\"IDA\n")])?;
    assert_eq!(p.run(Mode::All)?, Outcome::Finished);
    let log = p.read("migration.log");
    assert!(log.contains("Ejecutando comando: echo SAL\"\"IDA"), "{log}");
    assert!(
        !log.contains("SALIDA"),
        "la salida del comando va solo a la pantalla:\n{log}"
    );
    Ok(())
}
