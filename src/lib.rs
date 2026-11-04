//! stepwise: ejecutor de pasos de shell reanudable.
//!
//! Crate librería con toda la lógica. El binario (`main.rs`) solo la arranca.

pub mod app;
pub mod cli;
pub mod exec;
pub mod log;
pub mod menu;
pub mod parser;
pub mod runner;
pub mod state;

#[cfg(test)]
mod testutil;
