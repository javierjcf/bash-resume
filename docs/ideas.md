# Ideas y mejoras propuestas

Qué se podría añadir o mejorar para que bash-resume cumpla aún mejor su objetivo: **ejecutar procesos
largos paso a paso (migraciones de Odoo), con la seguridad de poder parar, arreglar y reanudar**.
Nada de esto está decidido: si se hace una, pasa a [tasks.md](tasks.md), con su decisión en
[decisions.md](decisions.md) si cambia el comportamiento.

## Valoración del estado actual

**Lo que funciona bien**
- El formato de pasos es bash normal con tres marcas (`#>`, `#.`, `#`): no hay que aprender nada nuevo
  y un fichero de pasos se puede probar a mano copiando líneas.
- Reanudar es fiable: la posición se guarda tras cada comando (de forma atómica), se valida contra el
  texto de la línea y el menú permite saltar a cualquier punto.
- Un único binario estático (con musl), sin Python ni dependencias en el servidor.
- Los comandos escriben directamente en la terminal (colores y barras de progreso), y Ctrl+C siempre los detiene.
- Se ve el progreso (`[13/40]`), cuánto queda según la ejecución anterior y lo que tarda cada fichero y el total.

**Dónde se queda corto para el objetivo**
- Que un fallo saque del programa es **intencionado**: imita a un script de bash y obliga a intervenir a mano.
  Para relanzar sin pasar por la interfaz ya existen `-y`/`-o`.
- **Dos rarezas heredadas de Python** (mantenidas a propósito): al reanudar en un `#.` se vuelve a parar,
  y al terminar todo, "Continuar" repite el último comando.

## Recomendadas (alto valor, poco código)

| Idea | Qué resuelve | Esfuerzo |
|---|---|---|
| **`[c] Continuar tras la parada`** en el popup cuando el estado está en una línea `#.` | Quita la rareza de la parada sin cambiar el comportamiento por defecto (no hace falta bajar a la línea siguiente) | Muy bajo |
| **Expandir `~`** en las rutas que se escriben en las preguntas | Hoy `~/migr` da "Ruta inválida" (el shell solo expande `~` en los argumentos, no en lo que se teclea) | Muy bajo |
| **Guardar una copia del estado corrupto** (`state.json.corrupto`) antes de recrearlo | Hoy se pierde la posición sin rastro; con la copia se puede recuperar a mano | Muy bajo |

## Útiles

| Idea | Qué resuelve | Esfuerzo |
|---|---|---|
| **Preguntar tras un Ctrl+C**: `[R] Reintentar \| [S] Saltar y seguir \| [N] Salir` | Salir al fallar es intencionado (como bash), así que esto solo tendría sentido como opción, sobre todo tras un Ctrl+C, que es una interrupción voluntaria. Encajaría como un bucle en `app` alrededor de `run_steps` | Bajo |
| **Más hooks junto a `onerror`**: `onstop` (al llegar a un `#.`) y `onfinish` (al terminar todo) | Avisos por Telegram al terminar o al llegar a una parada, sin escribir `telegram-send` en los pasos | Bajo |
| **`--list` / `--dry-run`**: mostrar qué se ejecutaría desde la posición actual, sin ejecutar | Revisar el plan antes de una migración real | Bajo |
| **`cd` persistente** con una marca `#cd dir` (cambia el directorio de las líneas siguientes) | Evitar repetir `cd odoo/custom && …` en muchas líneas | Bajo (solo el runner) |
| **Variables por proyecto**: un fichero `env` en el proyecto que se carga en cada comando | Nombres de BBDD, versiones… sin repetirlos; encaja bien con `-u` de bash | Bajo |
| **Historial de ejecución** (comando, fecha, duración, resultado de cada ejecución, no solo la última buena de `times.json`) | "¿Dónde falló la última vez?" y comparar ejecuciones | Medio |
| **Aviso de comando lento** en el resumen final: los que tardaron bastante más que la vez anterior | Detectar a tiempo que algo va peor que en el ensayo. Necesita guardar el tiempo anterior antes de sustituirlo en `times.json` | Bajo |
| **Menú: marcas de hecho/pendiente, búsqueda con `/`, `Inicio`/`Fin`/`RePág`** | Moverse rápido por ficheros largos y ver por dónde va la ejecución | Medio |
| **Separador de ejecución en el log** (`===== inicio: fecha, desde 02-migrar:3 =====`) | El log acumulado se lee mejor cuando hay muchas ejecuciones | Muy bajo |
| **Avanzar el estado aunque falle leer el fichero siguiente** | Hoy, si un comando va bien pero el fichero siguiente no se puede leer (no UTF-8, permisos), el estado no avanza y al reanudar se repite el comando | Bajo |

## Más ambiciosas

| Idea | Comentario |
|---|---|
| `bash-resume init DIR` | Crear el proyecto con una carpeta de pasos, un `onerror` de ejemplo y un README de los pasos |
| Timeout por paso (`#timeout 30m`) | Para pasos que se pueden colgar; habría que matar el grupo de procesos |
| Pasos condicionales o reintentos automáticos (`#retry 3`) | Útil para pasos de red inestables, pero acerca bash-resume a un sistema de tareas: valorar si compensa la complejidad |
| Ejecución remota (`--ssh host`) | Hoy se puede hacer con `ssh host bash-resume …`; integrarlo no aporta mucho |

## Calidad del proyecto

| Idea | Qué aporta |
|---|---|
| **Un tipo `Position` compartido** (fichero, línea y comando) | Hoy la misma idea tiene tres formas: tres `Option` sueltos en `State`, `runner::Position` y `menu::Choice`. Un único tipo haría que runner y menú devolvieran lo mismo (`Choice { position, mode }`), que `set_position` recibiera un valor y que el estado no pudiera tener "fichero sin línea" (`position: Option<Position>`). Sin romper `state.json`: `#[serde(flatten)]` + `#[serde(rename = "current_file")]` mantienen el JSON igual. Hacerlo antes de la próxima funcionalidad que toque posiciones (~30–40 min) |
| **Tests del binario completo** con `env!("CARGO_BIN_EXE_bash-resume")` (sin dependencias) | Probar códigos de salida y preguntas por tubería de punta a punta, como en la verificación manual |
| **CI en GitHub Actions**: `fmt --check`, `clippy -D warnings`, `test` en cada push | Que nada se rompa sin darse cuenta |
| **Releases con `cargo-dist`** (binario musl, `.tar.gz`, `install.sh`) y opcionalmente `.deb` con `cargo-deb` | Instalar en el equipo con una línea |
| `rust-version` en `Cargo.toml` | Declarar la versión mínima de Rust |
| Licencia (`LICENSE`) | El original es GPLv3; falta decidir la del proyecto nuevo |
