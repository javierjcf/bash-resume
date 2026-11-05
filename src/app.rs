//! Flujo principal: resolver rutas, preguntar qué hacer y lanzar el recorrido (spec §2, §7.1).

use std::io::{self, BufRead, Write};
use std::path::{self, Path, PathBuf};
use std::process::ExitCode;

use crate::cli::Args;
use crate::engine::runner::{Mode, Outcome, run_steps};
use crate::engine::state::State;
use crate::engine::timings::Timings;
use crate::log::Logger;
use crate::tui::{self, Choice};

/// Lo que el usuario decide hacer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Run(Mode),
    Exit,
}

/// Punto de entrada: resuelve rutas, pregunta qué hacer, ejecuta y devuelve el código de salida.
pub fn run(args: Args) -> io::Result<ExitCode> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout();

    let project = project_dir(args.project.as_deref())?;
    let state_path = project.join("state.json");
    if !state_path.exists() {
        writeln!(
            output,
            "No hay state.json en {}: se crea un proyecto nuevo.",
            project.display()
        )?;
    }
    let mut log = Logger::open(&project.join("migration.log"))?;
    let mut state = State::load(&state_path)?;
    log.info(&format!("Proyecto: {}", project.display()));

    let workdir = pick_dir(
        &[args.workdir.as_deref(), state.execution_path.as_deref()],
        "Introduce ruta válida para el directorio de trabajo:",
        &mut input,
        &mut output,
    )?;
    let steps = pick_dir(
        &[args.steps.as_deref(), state.steps_path.as_deref()],
        "Introduce ruta válida para la carpeta de pasos:",
        &mut input,
        &mut output,
    )?;
    log.info(&format!("Se usará execution_path: {}", workdir.display()));
    log.info(&format!("Se usará steps_path: {}", steps.display()));
    state.execution_path = Some(workdir.clone());
    state.steps_path = Some(steps.clone());
    state.save()?;

    let flag = match (args.yes, args.once) {
        (true, _) => Some(Mode::All),
        (_, true) => Some(Mode::Once),
        _ => None,
    };
    let mut timings = Timings::load(&project.join("times.json"));
    let action = choose_action(&mut state, &steps, flag, |steps, state| {
        tui::choose(steps, state, &timings)
    })?;
    // Se suelta stdin antes de ejecutar: a partir de aquí el teclado es para los comandos.
    drop(input);

    let Action::Run(mode) = action else {
        writeln!(output, "Salida solicitada. No se ejecuta nada.")?;
        return Ok(ExitCode::SUCCESS);
    };
    let onerror = project.join("onerror");
    let outcome = run_steps(
        &mut state,
        &steps,
        &workdir,
        &onerror,
        &mut log,
        mode,
        &mut timings,
    )?;
    report(outcome, &state, &mut output)
}

/// Carpeta del proyecto (donde viven state.json, times.json, migration.log y onerror), como ruta absoluta.
///
/// Es la indicada con `-p` o, si no hay, el directorio actual.
fn project_dir(arg: Option<&Path>) -> io::Result<PathBuf> {
    let dir = match arg {
        Some(dir) if dir.is_dir() => dir.to_path_buf(),
        Some(dir) => {
            return Err(io::Error::other(format!(
                "el proyecto {} no es un directorio",
                dir.display()
            )));
        }
        None => std::env::current_dir()?,
    };
    path::absolute(dir)
}

/// Mensaje final según cómo terminó el recorrido. Solo un fallo devuelve un código de error.
fn report(outcome: Outcome, state: &State, output: &mut impl Write) -> io::Result<ExitCode> {
    let command = state.last_command.as_deref().unwrap_or("");
    match outcome {
        Outcome::Finished => Ok(ExitCode::SUCCESS),
        Outcome::OnceDone => {
            writeln!(output, "Ejecución 'una vez' completada.")?;
            Ok(ExitCode::SUCCESS)
        }
        Outcome::Stopped => {
            writeln!(output, "Parada en línea de stop: {command}")?;
            Ok(ExitCode::SUCCESS)
        }
        Outcome::Failed => {
            writeln!(
                output,
                "Comando interrumpido o fallido: '{command}'. Estado no avanza."
            )?;
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Muestra `question` y devuelve la respuesta sin espacios.
///
/// Si la entrada se cierra (Ctrl+D), devuelve un error: no hay a quién preguntar.
fn ask(input: &mut impl BufRead, output: &mut impl Write, question: &str) -> io::Result<String> {
    write!(output, "{question} ")?;
    output.flush()?;
    let mut answer = String::new();
    if input.read_line(&mut answer)? == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "entrada cerrada",
        ));
    }
    Ok(answer.trim().to_string())
}

/// Devuelve, como ruta absoluta, el primer candidato que sea un directorio existente.
///
/// Si ninguno lo es, pregunta hasta recibir uno válido (como `set_path` en Python).
fn pick_dir(
    candidates: &[Option<&Path>],
    question: &str,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<PathBuf> {
    for dir in candidates.iter().flatten() {
        if dir.is_dir() {
            return path::absolute(dir);
        }
    }
    loop {
        let answer = ask(input, output, question)?;
        let dir = Path::new(&answer);
        if !answer.is_empty() && dir.is_dir() {
            return path::absolute(dir);
        }
        writeln!(output, "Ruta inválida, inténtalo de nuevo.")?;
    }
}

/// Decide qué ejecutar (spec §7).
///
/// Con `-y`/`-o` (`flag`) se continúa desde el estado sin abrir el menú, y el estado tiene que ser válido.
/// Sin flags se abre el menú, donde siempre se confirma en un popup antes de ejecutar.
/// `open_menu` es `menu::choose` en el programa y uno falso en los tests.
fn choose_action(
    state: &mut State,
    steps: &Path,
    flag: Option<Mode>,
    open_menu: impl FnOnce(&Path, &State) -> io::Result<Option<Choice>>,
) -> io::Result<Action> {
    if let Some(mode) = flag {
        if state.is_valid(steps) {
            return Ok(Action::Run(mode));
        }
        return Err(io::Error::other(
            "el estado no apunta a ninguna línea válida: lanza bash-resume sin -y/-o para elegirla",
        ));
    }
    match open_menu(steps, state)? {
        Some(choice) => {
            state.set_position(&choice.file, &choice.command, choice.line)?;
            Ok(Action::Run(choice.mode))
        }
        None => Ok(Action::Exit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;
    use std::fs;

    /// Carpeta con `steps/01-a` y un estado (en disco) que apunta a su segunda línea.
    fn project(name: &str) -> io::Result<(PathBuf, State)> {
        let dir = temp_dir(name)?;
        fs::create_dir(dir.join("steps"))?;
        fs::write(dir.join("steps/01-a"), "#> inicio\necho hola\n")?;
        let mut state = State::load(&dir.join("state.json"))?;
        state.set_position("01-a", "echo hola", 2)?;
        Ok((dir, state))
    }

    /// Menú falso que no debe abrirse.
    fn no_menu(_: &Path, _: &State) -> io::Result<Option<Choice>> {
        panic!("con -y/-o no se abre el menú")
    }

    #[test]
    fn menu_choice_is_saved_and_runs_in_its_mode() -> io::Result<()> {
        let (dir, mut state) = project("app-menu")?;
        let picked = Choice {
            file: "01-a".to_string(),
            line: 1,
            command: "#> inicio".to_string(),
            mode: Mode::Once,
        };

        let action = choose_action(&mut state, &dir.join("steps"), None, |_, _| {
            Ok(Some(picked))
        })?;

        assert_eq!(action, Action::Run(Mode::Once));
        assert_eq!(
            state.last_line_index,
            Some(1),
            "la elección se guarda en el estado"
        );
        assert_eq!(state.last_command.as_deref(), Some("#> inicio"));
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn leaving_the_menu_exits_without_changing_the_state() -> io::Result<()> {
        let (dir, mut state) = project("app-menu-quit")?;
        let action = choose_action(&mut state, &dir.join("steps"), None, |_, _| Ok(None))?;
        assert_eq!(action, Action::Exit);
        assert_eq!(state.last_line_index, Some(2));
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn yes_and_once_skip_the_menu() -> io::Result<()> {
        let (dir, mut state) = project("app-flags")?;
        let steps = dir.join("steps");
        assert_eq!(
            choose_action(&mut state, &steps, Some(Mode::All), no_menu)?,
            Action::Run(Mode::All)
        );
        assert_eq!(
            choose_action(&mut state, &steps, Some(Mode::Once), no_menu)?,
            Action::Run(Mode::Once)
        );
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn flags_with_invalid_state_are_an_error() -> io::Result<()> {
        let (dir, mut state) = project("app-flags-invalid")?;
        state.set_position("01-a", "comando que ya no existe", 2)?;
        let err =
            choose_action(&mut state, &dir.join("steps"), Some(Mode::All), no_menu).unwrap_err();
        assert!(err.to_string().contains("sin -y/-o"), "{err}");
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn closed_input_is_an_error() {
        let mut input: &[u8] = b"";
        let err = pick_dir(&[None], "¿Ruta?", &mut input, &mut Vec::new()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn pick_dir_uses_first_existing_candidate() -> io::Result<()> {
        let dir = temp_dir("app-pick")?;
        let missing = dir.join("no-existe");
        let candidates = [None, Some(missing.as_path()), Some(dir.as_path())];
        let mut input: &[u8] = b"";
        let picked = pick_dir(&candidates, "?", &mut input, &mut Vec::new())?;
        assert_eq!(picked, dir);
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn pick_dir_asks_until_valid() -> io::Result<()> {
        let dir = temp_dir("app-ask")?;
        let typed = format!("\n/no/existe\n{}\n", dir.display());
        let mut input = typed.as_bytes();
        let mut output = Vec::new();

        let picked = pick_dir(&[None], "¿Ruta?", &mut input, &mut output)?;

        assert_eq!(picked, dir);
        let shown = String::from_utf8_lossy(&output);
        assert_eq!(shown.matches("Ruta inválida").count(), 2, "{shown}");
        fs::remove_dir_all(&dir)
    }

    #[test]
    fn project_defaults_to_current_dir() -> io::Result<()> {
        assert_eq!(project_dir(None)?, std::env::current_dir()?);
        Ok(())
    }

    #[test]
    fn project_from_flag_is_made_absolute() -> io::Result<()> {
        // `cargo test` se ejecuta en la raíz del crate, donde existe `src/`.
        assert_eq!(
            project_dir(Some(Path::new("src")))?,
            std::env::current_dir()?.join("src")
        );
        Ok(())
    }

    #[test]
    fn project_flag_must_be_a_directory() {
        let err = project_dir(Some(Path::new("/no/existe"))).unwrap_err();
        assert!(err.to_string().contains("/no/existe"), "{err}");
    }

    #[test]
    fn only_failure_returns_error_code() -> io::Result<()> {
        let state = State::default();
        let mut out = Vec::new();
        assert_eq!(
            report(Outcome::Finished, &state, &mut out)?,
            ExitCode::SUCCESS
        );
        assert_eq!(
            report(Outcome::Stopped, &state, &mut out)?,
            ExitCode::SUCCESS
        );
        assert_eq!(
            report(Outcome::Failed, &state, &mut out)?,
            ExitCode::FAILURE
        );
        Ok(())
    }
}
