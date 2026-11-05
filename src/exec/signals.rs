//! Ctrl+C durante un comando: lo detiene a él, no a bash-resume (spec §4, T10).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use signal_hook::consts::SIGINT;
use signal_hook::flag;

/// Estado de Ctrl+C, compartido entre el programa y el manejador de señales del sistema operativo.
struct Sigint {
    /// `true` fuera de un comando: Ctrl+C hace lo de siempre (terminar bash-resume).
    terminate: Arc<AtomicBool>,
    /// Se pone a `true` cuando llega un Ctrl+C.
    received: Arc<AtomicBool>,
}

/// Registra los manejadores de SIGINT la primera vez que se llama y devuelve sus flags.
///
/// Las señales son de todo el proceso, así que esto es global por naturaleza: se registra una sola
/// vez y no se quita nunca (quitar un manejador de signal-hook no restaura el comportamiento por defecto).
/// Devuelve `None` si no se pudo registrar: entonces Ctrl+C simplemente termina bash-resume.
fn sigint() -> Option<&'static Sigint> {
    static SIGINT_FLAGS: OnceLock<Option<Sigint>> = OnceLock::new();
    SIGINT_FLAGS
        .get_or_init(|| {
            let terminate = Arc::new(AtomicBool::new(true));
            let received = Arc::new(AtomicBool::new(false));
            flag::register_conditional_default(SIGINT, Arc::clone(&terminate)).ok()?;
            flag::register(SIGINT, Arc::clone(&received)).ok()?;
            Some(Sigint {
                terminate,
                received,
            })
        })
        .as_ref()
}

/// Llamar justo antes de lanzar un comando: a partir de aquí, Ctrl+C no termina bash-resume.
pub(super) fn begin_command() {
    if let Some(s) = sigint() {
        s.received.store(false, Ordering::SeqCst);
        s.terminate.store(false, Ordering::SeqCst);
    }
}

/// Llamar al terminar el comando: Ctrl+C vuelve a terminar bash-resume. Devuelve si llegó alguno mientras tanto.
pub(super) fn end_command() -> bool {
    match sigint() {
        Some(s) => {
            s.terminate.store(true, Ordering::SeqCst);
            s.received.swap(false, Ordering::SeqCst)
        }
        None => false,
    }
}
