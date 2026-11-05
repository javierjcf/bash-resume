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
- Los comandos usan la terminal directamente: colores, barras de progreso y prompts funcionan.

**Dónde se queda corto para el objetivo**
- **Relanzar obliga a confirmar.** Que un fallo saque del programa es **intencionado**: imita a un script
  de bash y obliga a intervenir a mano. Lo que sobra es que, al volver, haya que pasar por la pregunta
  para continuar. Por lo mismo, tampoco se puede lanzar desatendido (script, `nohup`, `cron`) sin trucos
  como `printf 'y\n' |`. Lo resuelven los flags `-y`/`-o` (abajo).
- **Se sabe poco del progreso:** ni cuántos pasos quedan ni cuánto tarda cada fichero o la migración entera.
- **Dos rarezas heredadas de Python** (mantenidas a propósito): al reanudar en un `#.` se vuelve a parar,
  y al terminar todo, "Continuar" repite el último comando.

## Recomendadas (alto valor, poco código)

| Idea | Qué resuelve | Esfuerzo |
|---|---|---|
| **Relanzar sin preguntar: `-y/--yes` y `-o/--once`** | `-y` continúa desde el estado guardado sin preguntar; `-o` ejecuta un solo comando sin preguntar. Si el estado no es válido o falta una ruta, **error** (código 1) en vez de abrir el menú o preguntar: sin terminal no hay a quién preguntar. Tras arreglar un fallo basta con `bash-resume -y`, y vale para ejecuciones desatendidas. Encaje: dos flags en `cli.rs`, un atajo en `app::run` antes de `choose_action` y `pick_dir` sin preguntar cuando hay flag. ~40 líneas y tests | Bajo |
| **Pantalla única: resumen + menú en la TUI** (ver detalle abajo) | Sustituye la pregunta de texto `Y/M/O/N` por la TUI abierta directamente en la línea guardada: se ve el contexto y se decide con una tecla | Bajo-medio |
| **Tiempos por comando, por fichero y total** | Ya se registra `Tiempo de ejecución` de cada comando (`exec.rs`). Faltaría el tiempo de cada fichero al salir de él y el total de la ejecución al terminar, por el logger (pantalla y `migration.log`). Si termina con éxito el último fichero, un resumen con la tabla por fichero y el total. Encaje: `Instant` en `runner.rs` alrededor de cada fichero y de `run_steps`, reutilizando `format_duration`. **Limitación:** son tiempos de *esta* ejecución. Si hubo un fallo y se reanudó, el resumen no cubre lo anterior y no es representativo; es útil sobre todo con una migración ya ensayada que va de principio a fin. **Pendiente de decidir:** acumularlos entre reanudaciones (¿en `state.json`, o en un fichero aparte del proyecto?), poniéndolos a cero al empezar desde el principio. Relacionada con *Historial de ejecución* y *Progreso* | Bajo (sin acumular) / Medio (acumulando) |
| **`[C] Continuar tras la parada`** en la pregunta inicial cuando el estado está en una línea `#.` | Quita la rareza de la parada sin cambiar el comportamiento por defecto (no hace falta ir al menú) | Muy bajo |
| **Expandir `~`** en las rutas que se escriben en las preguntas | Hoy `~/migr` da "Ruta inválida" (el shell solo expande `~` en los argumentos, no en lo que se teclea) | Muy bajo |
| **Progreso**: `[12/40]` en el log y en el resumen | Saber cuánto queda en una migración larga | Bajo |
| **Guardar una copia del estado corrupto** (`state.json.corrupto`) antes de recrearlo | Hoy se pierde la posición sin rastro; con la copia se puede recuperar a mano | Muy bajo |

### Detalle: pantalla única (resumen + menú en la TUI)

Hoy hay dos interfaces para lo mismo (decidir desde qué línea ejecutar): la pregunta de texto
`[Y/S] Continuar | [M] Menú | [O] Una vez | [N] Salir` y, con `M`, el menú de ratatui. La propuesta es que
**el resumen y el menú sean la misma pantalla**: al arrancar se abre la TUI con el cursor ya en la línea
guardada.

```
┌ bash-resume ─────────────────────────────────────────────────────────────────┐
│ Workdir: /opt/odoo/doodba-20     Pasos: ~/migr/steps     ● Estado válido    │
├ Ficheros ───────────┬ 02-migrar ────────────────────────────────────────────┤
│   01-preparar       │  [001] #> MIGRANDO                                    │
│ > ● 02-migrar       │  [002] docker compose down || true                    │
│   03-limpiar        │> [003] docker compose run --rm odoo --update all ...  │
│                     │  [004] #. Revisar la BBDD                             │
├─────────────────────┴───────────────────────────────────────────────────────┤
│ ▶ 02-migrar:3 → docker compose run --rm odoo --update all --stop-after-init │
│ [Enter] continuar  [o] solo esta  [↑↓] elegir otra  [Tab] ficheros  [q] salir│
└─────────────────────────────────────────────────────────────────────────────┘
```

- **Cabecera:** workdir, carpeta de pasos y estado (`● válido` en verde o `⚠ no válido: elige una línea`
  en amarillo). Es la información del resumen de texto de hoy.
- **Centro:** el menú actual, con la línea guardada seleccionada y en amarillo.
- **Pie:** la línea que se va a ejecutar, bien visible, y las teclas resaltadas.
- **Teclas:** `Enter` sin moverse = el `Y` de hoy; `o` = `O`; moverse y `Enter` = lo que hoy es `M` + elegir;
  `q` = `N`. Desaparecen la pregunta `Y/M/O/N` y la tecla `M`.

**Ventajas**
- Una sola interfaz coherente, en vez de mezclar texto y TUI.
- Se ve el **contexto**: las líneas de alrededor y lo que viene después (por ejemplo, un `#.` dos líneas más abajo).
- Con estado inválido no hay paso intermedio (`[M] Abrir menú`): se elige directamente.
- `Menu` ya tiene la posición guardada, el estado y los paneles: es sobre todo dibujo (cabecera y pie) y
  cambiar `app` para abrir la TUI en lugar de preguntar. ~60–100 líneas.

**A tener en cuenta**
- **Sin terminal no hay TUI.** Hoy el flujo de texto funciona por tubería (`printf 'y\n' |`). Va de la mano de
  `-y`/`-o`: TUI cuando estás delante y flags cuando no (o mantener la pregunta de texto solo como plan B).
  Conviene hacerla junto con los flags o justo después.
- **La pantalla alternativa no deja rastro en el scroll** de la terminal (hoy se puede subir y ver el resumen
  confirmado). Se compensa registrando en el log la elección: `▶ Ejecutando desde 02-migrar:3 (continuar)`.
- Se pierde la confirmación en dos pasos (ver el resumen y luego confirmar): basta un `Enter`. Con la línea
  destacada en el pie parece suficiente; si se quisiera más seguridad, pedir `Enter` dos veces solo cuando
  se ha movido el cursor de la línea guardada.
- Cambia el comportamiento de la spec (§7.1 y §7.2) y la decisión D5: habría que reflejarlo en `decisions.md`.

## Útiles

| Idea | Qué resuelve | Esfuerzo |
|---|---|---|
| **Preguntar tras un Ctrl+C**: `[R] Reintentar \| [S] Saltar y seguir \| [N] Salir` | Salir al fallar es intencionado (como bash), así que esto solo tendría sentido como opción, sobre todo tras un Ctrl+C, que es una interrupción voluntaria. Encajaría como un bucle en `app` alrededor de `run_steps` | Bajo |
| **Más hooks junto a `onerror`**: `onstop` (al llegar a un `#.`) y `onfinish` (al terminar todo) | Avisos por Telegram al terminar o al llegar a una parada, sin escribir `telegram-send` en los pasos | Bajo |
| **`--list` / `--dry-run`**: mostrar qué se ejecutaría desde la posición actual, sin ejecutar | Revisar el plan antes de una migración real | Bajo |
| **`cd` persistente** con una marca `#cd dir` (cambia el directorio de las líneas siguientes) | Evitar repetir `cd odoo/custom && …` en muchas líneas | Bajo (solo el runner) |
| **Variables por proyecto**: un fichero `env` en el proyecto que se carga en cada comando | Nombres de BBDD, versiones… sin repetirlos; encaja bien con `-u` de bash | Bajo |
| **Historial de ejecución** en `state.json` (comando, fecha, duración, resultado) | "¿Dónde falló la última vez?" y estimar cuánto tarda cada paso. Python ignora las claves extra, así que sigue siendo compatible | Medio |
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
| **Un tipo `Position` compartido** (fichero, línea y comando) | Hoy la misma idea tiene tres formas: tres `Option` sueltos en `State`, `runner::Position` y `menu::Choice`. Un único tipo haría que runner y menú devolvieran lo mismo (`Choice { position, mode }`), que `set_position` recibiera un valor y que el estado no pudiera tener "fichero sin línea" (`position: Option<Position>`). Sin romper `state.json`: `#[serde(flatten)]` + `#[serde(rename = "current_file")]` mantienen el JSON igual. Hacerlo justo antes de `-y`/`-o` o de la pantalla única (~30–40 min) |
| **Estructura de módulos** | Plana mientras cada módulo se lea de un tirón (hoy ~1.200 líneas de código en 9 módulos). Dividir solo el que crezca: `menu.rs` primero (355 líneas de código; con la pantalla única pasaría de ~500), en `menu.rs` (estado y teclas) + `menu/render.rs` (dibujo), sin cambiar los `use` del resto. Helpers compartidos de los tests de integración en `tests/common/mod.rs`. Un *workspace* con varios crates solo si una parte se reutiliza en otro programa |
| **Tests del binario completo** con `env!("CARGO_BIN_EXE_bash-resume")` (sin dependencias) | Probar códigos de salida y preguntas por tubería de punta a punta, como en la verificación manual |
| **CI en GitHub Actions**: `fmt --check`, `clippy -D warnings`, `test` en cada push | Que nada se rompa sin darse cuenta |
| **Releases con `cargo-dist`** (binario musl, `.tar.gz`, `install.sh`) y opcionalmente `.deb` con `cargo-deb` | Instalar en el equipo con una línea |
| `rust-version` en `Cargo.toml` | Declarar la versión mínima de Rust |
| Licencia (`LICENSE`) | El original es GPLv3; falta decidir la del proyecto nuevo |
