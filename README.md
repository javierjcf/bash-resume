# stepwise

Ejecuta comandos de shell desde **ficheros de pasos**, línea a línea, recordando por dónde va.
Si algo falla, puedes **reanudar** desde ese punto o elegir otro en un menú interactivo.

Pensado para procesos largos y repetibles (por ejemplo, migraciones de Odoo), donde un script
de bash con `set -e` se para, pero no sabe continuar.

> 🚧 **En desarrollo.** Reescritura en Rust de [pyBashResume](https://github.com/Comunitea/pyBashResume).
> Progreso en [docs/tasks.md](docs/tasks.md). Lo de abajo describe el comportamiento objetivo ([docs/spec.md](docs/spec.md)).

## Uso

```bash
stepwise [WORKDIR] [-s|--steps DIR] [-H|--home DIR]
```

| Argumento | Qué es | Si no se indica |
|---|---|---|
| `WORKDIR` | Directorio donde se ejecutan los comandos | El guardado en el estado, o se pregunta |
| `-s, --steps` | Carpeta con los ficheros de pasos | La guardada, `migration-steps`, o se pregunta |
| `-H, --home` | Dónde viven `state.json`, `migration.log` y `onerror` | `$STEPWISE_HOME`, o se pregunta |

Al arrancar muestra el punto pendiente y pregunta:
`[Y/S] Continuar | [M] Menú | [O] Una vez | [N] Salir`.

## Ficheros de pasos

Se ejecutan en **orden alfabético** (`01-setup`, `02-migrar`…), ignorando los ocultos. Cada línea es
un proceso `sh -c` independiente: un `cd` no se mantiene en la línea siguiente (usa `cd dir && …`).

```bash
#> PREPARANDO ENTORNO              ← mensaje al log
docker compose down || true
git pull origin master
# esto es un comentario             ← se ignora
#. Revisar la BBDD antes de seguir  ← se detiene aquí
docker compose run --rm odoo --stop-after-init
```

- **Si un comando falla**, se guarda el estado en esa línea y se ejecuta el fichero `onerror`
  (por ejemplo `telegram-send "❌ Error"`).
- **`once`** (`O`) ejecuta un solo comando y se para, para avanzar paso a paso.

## Instalación

```bash
cargo install --path .
```

El binario se instala en `~/.cargo/bin/stepwise`.

## Desarrollo

```bash
cargo run -- --help    # compilar y ejecutar
cargo test             # tests
cargo clippy --all-targets -- -D warnings
cargo fmt
```

Más comandos en [docs/cheatsheet.md](docs/cheatsheet.md). Cómo trabajamos: [docs/README.md](docs/README.md).
