//! Argumentos de línea de comandos (spec §9).

use std::path::PathBuf;

use clap::Parser;

/// Ejecuta ficheros de pasos de shell línea a línea, recordando por dónde va para poder reanudar.
#[derive(Debug, Parser)]
#[command(version)]
pub struct Args {
    /// Directorio donde se ejecutan los comandos (por defecto, el guardado en el estado)
    pub workdir: Option<PathBuf>,

    /// Carpeta con los ficheros de pasos (por defecto, la guardada en el estado)
    #[arg(short, long, value_name = "DIR")]
    pub steps: Option<PathBuf>,

    /// Carpeta del proyecto: state.json, migration.log y onerror (por defecto, el directorio actual)
    #[arg(short, long, value_name = "DIR")]
    pub project: Option<PathBuf>,

    /// Continúa desde la posición guardada sin abrir el menú
    #[arg(short, long)]
    pub yes: bool,

    /// Ejecuta solo el comando de la posición guardada, sin abrir el menú
    #[arg(short, long, conflicts_with = "yes")]
    pub once: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn clap_definition_is_valid() {
        Args::command().debug_assert();
    }

    #[test]
    fn no_args_leaves_paths_unset() {
        let args = Args::try_parse_from(["bash-resume"]).unwrap();
        assert_eq!(args.workdir, None);
        assert_eq!(args.steps, None);
        assert_eq!(args.project, None);
    }

    #[test]
    fn parses_short_flags() {
        let args = Args::try_parse_from(["bash-resume", "/w", "-s", "/s", "-p", "/p"]).unwrap();
        assert_eq!(args.workdir, Some(PathBuf::from("/w")));
        assert_eq!(args.steps, Some(PathBuf::from("/s")));
        assert_eq!(args.project, Some(PathBuf::from("/p")));
    }

    #[test]
    fn parses_long_flags() {
        let args =
            Args::try_parse_from(["bash-resume", "--steps", "/s", "--project", "/p"]).unwrap();
        assert_eq!(args.steps, Some(PathBuf::from("/s")));
        assert_eq!(args.project, Some(PathBuf::from("/p")));
    }

    #[test]
    fn parses_yes_and_once_flags() {
        let args = Args::try_parse_from(["bash-resume", "-y"]).unwrap();
        assert!(args.yes && !args.once);
        let args = Args::try_parse_from(["bash-resume", "--once"]).unwrap();
        assert!(args.once && !args.yes);
        assert!(
            Args::try_parse_from(["bash-resume", "-y", "-o"]).is_err(),
            "son incompatibles"
        );
    }

    #[test]
    fn rejects_unknown_flag() {
        assert!(Args::try_parse_from(["bash-resume", "--steps-path", "/s"]).is_err());
        assert!(
            Args::try_parse_from(["bash-resume", "--home", "/h"]).is_err(),
            "--home ya no existe"
        );
    }
}
