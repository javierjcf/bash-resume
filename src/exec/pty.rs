//! Terminal falsa (PTY) vacía para el stdin de los comandos (D15).
//!
//! Es una terminal, así que los comandos que la exigen (`docker run -it`) funcionan; pero no está conectada al
//! teclado: si un comando la pone en modo raw, la terminal real sigue igual y Ctrl+C sigue llegando como señal a
//! bash-resume. No se lee ni se reenvía nada: solo se entrega como stdin.

use std::io;
use std::os::fd::OwnedFd;

use rustix::fs::{Mode, OFlags, open};
use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};

/// Crea la terminal y devuelve `(maestro, esclavo)`. El esclavo es el stdin del comando; el maestro tiene que seguir
/// abierto hasta que el comando termine (al cerrarlo, el esclavo deja de ser una terminal).
///
/// Lleva ya escrito un fin de entrada (Ctrl+D): quien lea stdin recibe EOF en vez de quedarse esperando.
pub(super) fn empty_tty() -> io::Result<(OwnedFd, OwnedFd)> {
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC)?;
    grantpt(&master)?;
    unlockpt(&master)?;
    // NOCTTY: que no se convierta en la terminal de control de bash-resume.
    let slave = open(
        ptsname(&master, Vec::new())?,
        OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    rustix::io::write(&master, b"\x04")?;
    Ok((master, slave))
}
