//! Ejecución de comandos: lanzar bash y gestionar Ctrl+C mientras corren.

mod command;
mod signals;

pub use command::execute;
