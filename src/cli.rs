//! Argumentos de línea de comandos (spec §9).

use std::path::PathBuf;

use clap::Parser;

/// Ejecuta ficheros de pasos de shell línea a línea, recordando por dónde va para poder reanudar.
#[derive(Debug, Parser)]
#[command(version)]
pub struct Args {
    /// Directorio donde se ejecutan los comandos (por defecto, el guardado en el estado)
    pub workdir: Option<PathBuf>,

    /// Carpeta con los ficheros de pasos (por defecto, la guardada o `migration-steps`)
    #[arg(short, long, value_name = "DIR")]
    pub steps: Option<PathBuf>,

    /// Directorio con state.json, migration.log y onerror
    #[arg(short = 'H', long, value_name = "DIR", env = "STEPWISE_HOME")]
    pub home: Option<PathBuf>,
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
        let args = Args::try_parse_from(["stepwise"]).unwrap();
        assert_eq!(args.workdir, None);
        assert_eq!(args.steps, None);
        // `home` no se comprueba: depende de si STEPWISE_HOME está definida en el entorno.
    }

    #[test]
    fn parses_short_flags() {
        let args = Args::try_parse_from(["stepwise", "/w", "-s", "/s", "-H", "/h"]).unwrap();
        assert_eq!(args.workdir, Some(PathBuf::from("/w")));
        assert_eq!(args.steps, Some(PathBuf::from("/s")));
        assert_eq!(args.home, Some(PathBuf::from("/h")));
    }

    #[test]
    fn parses_long_flags() {
        let args = Args::try_parse_from(["stepwise", "--steps", "/s", "--home", "/h"]).unwrap();
        assert_eq!(args.steps, Some(PathBuf::from("/s")));
        assert_eq!(args.home, Some(PathBuf::from("/h")));
    }

    #[test]
    fn rejects_unknown_flag() {
        assert!(Args::try_parse_from(["stepwise", "--steps-path", "/s"]).is_err());
    }
}
