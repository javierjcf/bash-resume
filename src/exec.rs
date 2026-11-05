//! Ejecución de comandos: lanzar bash y gestionar Ctrl+C mientras corren.

mod command;
mod pty;
mod signals;

pub use command::{Execution, Progress, execute};
