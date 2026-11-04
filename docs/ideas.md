# Ideas y mejoras propuestas

Qué se podría añadir o mejorar para que stepwise cumpla aún mejor su objetivo: **ejecutar procesos
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
- **Un fallo te saca del programa.** Hay que relanzar, ver el resumen y volver a confirmar. En una
  migración de horas, con muchos reintentos, eso son pasos de más.
- **No sirve para ejecuciones desatendidas.** Siempre pregunta al arrancar, así que no se puede lanzar
  desde un script, `nohup` o `cron` sin trucos (`printf 'y\n' |`).
- **Se sabe poco del progreso:** ni cuántos pasos quedan ni cuánto tardó la última vez cada uno.
- **Dos rarezas heredadas de Python** (mantenidas a propósito): al reanudar en un `#.` se vuelve a parar,
  y al terminar todo, "Continuar" repite el último comando.

## Recomendadas (alto valor, poco código)

| Idea | Qué resuelve | Esfuerzo |
|---|---|---|
| **Preguntar tras un fallo o un Ctrl+C**: `[R] Reintentar \| [S] Saltar y seguir \| [M] Menú \| [N] Salir` | El ciclo "falla → arreglo → reintento" sin salir del programa. `onerror` se lanzaría solo al salir (o siempre, configurable). Encaja como un bucle en `app` alrededor de `run_steps` | Bajo |
| **`[C] Continuar tras la parada`** en la pregunta inicial cuando el estado está en una línea `#.` | Quita la rareza de la parada sin cambiar el comportamiento por defecto (no hace falta ir al menú) | Muy bajo |
| **Modo no interactivo**: `--yes` (continuar sin preguntar) y `--once` | Ejecuciones desatendidas (`tmux`, `nohup`, CI). Con estado inválido, error en vez de menú | Bajo |
| **Expandir `~`** en las rutas que se escriben en las preguntas | Hoy `~/migr` da "Ruta inválida" (el shell solo expande `~` en los argumentos, no en lo que se teclea) | Muy bajo |
| **Progreso**: `[12/40]` en el log y en el resumen, y tiempo total al terminar | Saber cuánto queda en una migración larga | Bajo |
| **Guardar una copia del estado corrupto** (`state.json.corrupto`) antes de recrearlo | Hoy se pierde la posición sin rastro; con la copia se puede recuperar a mano | Muy bajo |

## Útiles

| Idea | Qué resuelve | Esfuerzo |
|---|---|---|
| **Más hooks junto a `onerror`**: `onstop` (al llegar a un `#.`) y `onfinish` (al terminar todo) | Avisos por Telegram al terminar o al llegar a una parada, sin escribir `telegram-send` en los pasos | Bajo |
| **`--list` / `--dry-run`**: mostrar qué se ejecutaría desde la posición actual, sin ejecutar | Revisar el plan antes de una migración real | Bajo |
| **`cd` persistente** con una marca `#cd dir` (cambia el directorio de las líneas siguientes) | Evitar repetir `cd odoo/custom && …` en muchas líneas | Bajo (solo el runner) |
| **Variables por proyecto**: un fichero `env` en el home que se carga en cada comando | Nombres de BBDD, versiones… sin repetirlos; encaja bien con `-u` de bash | Bajo |
| **Historial de ejecución** en `state.json` (comando, fecha, duración, resultado) | "¿Dónde falló la última vez?" y estimar cuánto tarda cada paso. Python ignora las claves extra, así que sigue siendo compatible | Medio |
| **Menú: marcas de hecho/pendiente, búsqueda con `/`, `Inicio`/`Fin`/`RePág`** | Moverse rápido por ficheros largos y ver por dónde va la ejecución | Medio |
| **Separador de ejecución en el log** (`===== inicio: fecha, desde 02-migrar:3 =====`) | El log acumulado se lee mejor cuando hay muchas ejecuciones | Muy bajo |
| **Avanzar el estado aunque falle leer el fichero siguiente** | Hoy, si un comando va bien pero el fichero siguiente no se puede leer (no UTF-8, permisos), el estado no avanza y al reanudar se repite el comando | Bajo |

## Más ambiciosas

| Idea | Comentario |
|---|---|
| `stepwise init DIR` | Crear el home con `steps/`, un `onerror` de ejemplo y un README de los pasos |
| Timeout por paso (`#timeout 30m`) | Para pasos que se pueden colgar; habría que matar el grupo de procesos |
| Pasos condicionales o reintentos automáticos (`#retry 3`) | Útil para pasos de red inestables, pero acerca stepwise a un sistema de tareas: valorar si compensa la complejidad |
| Ejecución remota (`--ssh host`) | Hoy se puede hacer con `ssh host stepwise …`; integrarlo no aporta mucho |

## Calidad del proyecto

| Idea | Qué aporta |
|---|---|
| **Tests del binario completo** con `env!("CARGO_BIN_EXE_stepwise")` (sin dependencias) | Probar códigos de salida y preguntas por tubería de punta a punta, como en la verificación manual |
| **CI en GitHub Actions**: `fmt --check`, `clippy -D warnings`, `test` en cada push | Que nada se rompa sin darse cuenta |
| **Releases con `cargo-dist`** (binario musl, `.tar.gz`, `install.sh`) y opcionalmente `.deb` con `cargo-deb` | Instalar en el equipo con una línea |
| `rust-version` en `Cargo.toml` | Declarar la versión mínima de Rust |
| Licencia (`LICENSE`) | El original es GPLv3; falta decidir la del proyecto nuevo |
