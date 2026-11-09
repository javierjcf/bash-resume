# AGENTS.md — bash-resume

Ejecutor de pasos de shell **reanudable**: ejecuta ficheros de pasos línea a línea, recuerda por dónde va y
permite reanudar o elegir otra línea en una interfaz de terminal. Es una reescritura en Rust de pyBashResume
(Python), de la que ya se separa a propósito en varias cosas (ver `docs/decisions.md`).

## Antes de empezar

1. El comportamiento vigente está en [docs/spec.md](docs/spec.md), la organización del código en
   [docs/architecture.md](docs/architecture.md) y las decisiones en [docs/decisions.md](docs/decisions.md).
   No los contradigas sin proponer antes un cambio.
2. El trabajo va por tareas en [docs/tasks.md](docs/tasks.md); las propuestas sin decidir, en
   [docs/ideas.md](docs/ideas.md). Una idea se convierte en tarea antes de programarla.

## Flujo de trabajo

- Al cerrar una tarea: márcala `[x]` en `tasks.md`. Si cambia el comportamiento, actualiza `docs/spec.md`, el
  `README.md`, `docs/cheatsheet.md` y la sección `[Unreleased]` de `CHANGELOG.md`.
- Una decisión nueva se añade al final de `decisions.md` con el siguiente número; una que cambia no se borra,
  se marca *Sustituida por Dn*.
- Haz commits solo cuando el usuario lo pida.
- Releases: [docs/release.md](docs/release.md). Las lanza el usuario con `scripts/release.sh`; no subas tags.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/es/): `tipo(ámbito): descripción` en imperativo,
con pie `Refs: Tn` cuando hay tarea. Tipos: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`. Si cambia la CLI
de forma incompatible: `tipo(ámbito)!:` y pie `BREAKING CHANGE: …`.

## Convenciones

- Identificadores en inglés; comentarios, `///`, mensajes al usuario y docs en español.
- Sin `unsafe`. Sin `unwrap()`/`expect()` fuera de los tests; propaga errores con `?`.
- Pocas dependencias: no añadas crates sin proponerlo y anotarlo en `decisions.md`.
- Antes de dar algo por terminado: `cargo fmt`, `cargo clippy --all-targets -- -D warnings` y `cargo test`.

## Probar

- La lógica de la TUI (`Menu::handle_key`) se testea sin terminal; el dibujo, con `ratatui::backend::TestBackend`
  (se dibuja y se comprueba el texto).
- El comportamiento real con teclas (popup, `Esc`, ejecutar) solo se puede comprobar ejecutando el binario en un
  PTY; los tests no lo cubren.
- Los comandos de los pasos se ejecutan con `bash -euo pipefail` en el directorio de trabajo: los tests que lanzan
  procesos usan comandos silenciosos (`true`, `false`, `echo x >> out`) y un directorio temporal propio.
