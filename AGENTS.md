# AGENTS.md — bash-resume

Ejecutor de pasos de shell **reanudable**, reescrito en Rust a partir de `pyBashResume/` (el original en Python,
solo de consulta e ignorado por git). Este proyecto es también un **proyecto de aprendizaje de Rust**.

## Antes de empezar cualquier sesión

1. Lee [docs/tasks.md](docs/tasks.md) y localiza la tarea `[~]` o la siguiente `[ ]`. Si están todas hechas, las propuestas están en [docs/ideas.md](docs/ideas.md): una idea se convierte en tarea antes de programarla.
2. El comportamiento está en [docs/spec.md](docs/spec.md), la organización del código en [docs/architecture.md](docs/architecture.md) y las decisiones en [docs/decisions.md](docs/decisions.md). No las contradigas sin proponer antes un cambio.
3. Si hay dudas sobre el comportamiento original, consulta `pyBashResume/*.py`.

## Modo de trabajo: enseñanza

El usuario sabe algo de Rust (leyó el Rust Book) pero tiene poca práctica.
- **Una tarea por iteración**, en pasos pequeños que compilen. No adelantes código de tareas futuras.
- **Explica cada concepto de Rust nuevo** cuando aparezca: qué es, por qué se usa aquí, la alternativa y la referencia al Rust Book. Si un error del compilador es didáctico, enséñalo.
- Prefiere código **idiomático pero simple** frente a abstracciones genéricas.
- Al cerrar una tarea: márcala `[x]` en `tasks.md`, añade su entrada en `docs/learning.md` y actualiza la spec si algo ha cambiado.
- Haz commits solo cuando el usuario lo pida.

## Convenciones

- Identificadores en inglés; comentarios, `///`, mensajes y docs en español.
- Sin `unsafe`. Sin `unwrap()`/`expect()` fuera de los tests; propaga errores con `?`.
- Pocas dependencias: no añadas crates sin proponerlo y anotarlo en `decisions.md`.
- Antes de dar algo por terminado: `cargo fmt`, `cargo clippy -- -D warnings` y `cargo test`.

## Comandos

```bash
cargo check            # rápido, solo tipos
cargo test             # tests
cargo clippy -- -D warnings
cargo run -- --help
```
