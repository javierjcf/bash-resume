# Especificación funcional — bash-resume

Qué hace el programa, visto desde fuera. No describe cómo está hecho en Rust: eso va en
[decisions.md](decisions.md) y en el código. La referencia es la versión Python en `pyBashResume/`.
Las diferencias deliberadas están marcadas con **[cambio]**.

## 1. Concepto

`bash-resume` ejecuta, en orden, las líneas de una serie de ficheros de pasos. Cada línea es un
comando de shell independiente. Recuerda por dónde va y permite reanudar desde el último fallo,
o desde cualquier punto elegido en un menú interactivo.

## 2. Directorios

| Nombre | Contenido | Cómo se resuelve |
|---|---|---|
| **proyecto** | `state.json`, `migration.log`, `onerror` | `-p/--project DIR` → directorio actual. No se guarda en el estado. **[cambio]** En Python era el directorio del script (D11). |
| **workdir** (`execution_path` en el estado) | Directorio donde se ejecutan los comandos | argumento posicional `WORKDIR` → `state.json` → pregunta interactiva |
| **steps** (`steps_path` en el estado) | Carpeta con los ficheros de pasos | `-s/--steps` → `state.json` → pregunta interactiva |

- **[cambio]** No hay estructura impuesta: en Python, `--steps-path` tenía `default="migration-steps"`; aquí la carpeta de pasos y el directorio de trabajo se indican la primera vez (argumento o pregunta) y quedan guardados.
- Si en el proyecto no hay `state.json`, se avisa (`No hay state.json en <ruta>: se crea un proyecto nuevo.`) y se crea; si está corrupto, se avisa y se recrea. En ambos casos se preguntan las rutas que falten.
- `-p` con una ruta que no es un directorio: `❌ Error: el proyecto <ruta> no es un directorio` y código 1. No se buscan proyectos en carpetas superiores.
- Una ruta es válida si existe. Se guarda como ruta absoluta.
- Una ruta inválida en la pregunta interactiva muestra un error y se vuelve a pedir.

## 3. Ficheros de pasos

- Son los ficheros de `steps_path` cuyo nombre **no empieza por `.`**, en **orden alfabético**.
- **[cambio]** Se ignoran los subdirectorios (en Python se incluían y fallaba al intentar leerlos) y los nombres que no son UTF-8.
- Se aceptan finales de línea `\n` y `\r\n` (en Python, el `\r` quedaba pegado al comando).
- Hay un ejemplo en `demo/steps/`.
- Cada línea se clasifica (sobre la línea sin espacios al inicio ni al final) por la **primera** regla que cumpla:

| Prefijo | Tipo | Efecto |
|---|---|---|
| (vacía) | `empty` | se ignora |
| `#>` | `log` | se escribe en el log (nivel INFO) y no se ejecuta |
| `#.` | `stop` | se escribe en el log (WARNING), se guarda el estado en esa línea y se **detiene** |
| `#` | `comment` | se ignora |
| otra | `cmd` | se ejecuta |

- El texto que se guarda y ejecuta es la línea original (sin el `\n`).
- Los números de línea son **1-based** y cuentan todas las líneas, incluidas vacías y comentarios.

## 4. Ejecución de un comando

- Se ejecuta como `bash -euo pipefail -c "<línea>"` en `execution_path`. Cada línea es un proceso nuevo, así que un `cd` no afecta a la siguiente.
- **[cambio]** En Python era `sh -c` (`shell=True`). Ahora es bash en modo estricto (D10):
  - se puede usar sintaxis de bash (`[[ ]]`, arrays, `{a,b}`, `<(…)`, `source`…);
  - `-e`: si una parte de la línea falla (`a; b`), se para ahí y la línea cuenta como fallo;
  - `-u`: usar una variable sin definir es un error (para un valor por defecto: `${VAR:-valor}`);
  - `-o pipefail`: `a | b` falla si falla `a`, no solo si falla `b`.
  Para aceptar un fallo concreto: `cmd || true`.
- **[cambio]** No se hace `chdir` en el propio programa: cada comando se lanza con ese directorio de trabajo (D8). El comando ve lo mismo.
- Si el comando no se puede lanzar (por ejemplo, el directorio no existe), cuenta como fallo.
- La **salida** del comando va directamente a la terminal (stdout y stderr heredados): aparece en tiempo real con sus colores y barras de progreso. **[cambio]** En Python se usaba un PTY.
- El comando **no recibe el teclado**: stdin es una terminal falsa (PTY) vacía (D15). Es una terminal, así que `docker run -it` y `docker compose run` funcionan; pero si un comando la pone en modo raw no afecta a la terminal real, y Ctrl+C sigue llegando como señal a bash-resume. Un comando que lea de stdin recibe fin de entrada; `sudo` sigue pidiendo la contraseña porque la lee de `/dev/tty`. Si no se puede crear la PTY, stdin es `/dev/null`.
- **¿Es un error?** `exit 0` → éxito; cualquier otro código (o morir por una señal) → fallo.
  **[cambio]** Se elimina la heurística de Python para `exit 255` + `--stop-after-init`: en Odoo un 255 es siempre un error real (D9). Para aceptar un código en un paso concreto: `cmd || true`.
- **Ctrl+C** durante un comando: la terminal se lo envía al comando, que se detiene; bash-resume **no** se detiene, registra `⛔ … (Ctrl+C)` y lo trata como un fallo (estado en esa línea + `onerror`, código 1), igual que Python. Cuenta como fallo aunque el comando termine con 0 al recibirlo (Odoo hace `os._exit(0)` al segundo Ctrl+C).
- En el log, **dos líneas por comando** (§8): al empezar, el comando con su número entre todos los de los ficheros de pasos (contando desde el principio, no desde donde se reanuda) y, si hay tiempos guardados, lo que queda (§6.1); al terminar, el resultado, lo que tardó (`HH:MM:SS`) y de nuevo el comando, para que cada línea se entienda sola. **[cambio]** En Python eran cuatro (`Ejecutando comando`, `Proceso finalizado…`, `Comando completado…` y `Tiempo de ejecución: h:m:s`).

## 5. Recorrido de los pasos (`run_steps`)

1. Se empieza en `current_file` (si existe; si no, en el primer fichero) y en la línea `last_line_index` de ese fichero. Los ficheros siguientes empiezan en su línea 1.
2. Para cada línea:
   - `empty` / `comment` → se salta.
   - `log` → se escribe en el log.
   - `stop` → se guarda el estado en esa línea, se registra como WARNING y **fin**. Al reanudar con "Continuar" se vuelve a parar en el mismo `#.` (como en Python); para pasarlo hay que elegir otra línea en el menú.
   - `cmd` → se ejecuta:
     - **Fallo:** se guarda el estado en esa línea, se ejecuta `onerror` y **fin**.
     - **Éxito:** el estado avanza al **siguiente `cmd`** (en este fichero o en los siguientes). Si no hay más, se queda en el que se acaba de ejecutar. En modo **una vez**, **fin** tras este primer `cmd` ejecutado.
3. Cuando se terminan los ficheros: `Procesado todos los ficheros disponibles.`
4. El recorrido devuelve cómo terminó (terminado, parada, fallo o una vez) para que el flujo principal decida qué mostrar.
5. Cada fichero se lee al llegar a él, así que se pueden editar los pasos siguientes mientras se ejecutan los anteriores.
6. Tiempos (§6.1), sin cambiar nada de lo anterior: cada `cmd` que termina bien guarda lo que tardó; al acabar cada fichero se registra `🕒 Fichero 02-migrar: 01:05:30`; al final, `🕒 Esta ejecución: …` y, solo si se procesaron todos los ficheros, `🕒 Resumen de tiempos:` con una fila por fichero y el total.

### onerror

- Es el fichero `onerror` dentro del **proyecto**. Si no existe → WARNING en el log.
- Se ejecuta línea a línea, saltando las vacías y las que empiezan por `#`. Sus fallos se ignoran. En el log, cada comando sale como los demás (`⏳ comando` y su resultado), sin `[n/N]` y con la fuente `[onerror]`.

## 6. Estado (`state.json` en el proyecto)

El formato es **compatible con la versión Python**:

```json
{
    "execution_path": "/abs/path" | null,
    "steps_path": "/abs/path" | null,
    "current_file": "02-setup" | null,
    "last_command": "docker compose up -d" | null,
    "last_line_index": 5 | null
}
```

- Si no existe, se crea con todo a `null`. Si está corrupto, se avisa y se recrea. Si le faltan claves, se completan con `null`.
- Se guarda **en cada cambio**, de forma atómica (se escribe `state.json.tmp` y se renombra): si el proceso muere a mitad, queda el estado anterior, nunca uno a medias.
- **Válido** si: `current_file`, `last_line_index` y `last_command` tienen valor; el fichero existe; la línea existe, y su texto (sin espacios al inicio ni al final) coincide con `last_command`.
- **[cambio]** `last_line_index = 0` no es válido (en Python, `lines[0 - 1]` apuntaba a la última línea).
- Rust escribe el JSON con 2 espacios de sangría y Python con 4; los dos leen cualquiera.

### 6.1 Tiempos (`times.json` en el proyecto) **[nuevo]**

```json
{
  "09-mig": {
    "docker compose -f devel.yaml build --pull": "00:05:12",
    "docker compose -f devel.yaml run --rm odoo --update all --stop-after-init": "08:59:12"
  },
  "13-mig": {
    "docker compose -f devel.yaml run --rm odoo --update all --stop-after-init": "02:19:37"
  }
}
```

- Guarda lo que tardó (`HH:MM:SS`) **la última vez que terminó bien** cada comando, agrupado por fichero de pasos. Dentro de cada fichero, la clave es el texto del comando sin espacios al inicio ni al final, sin número de línea (D17). El mismo comando en dos ficheros tiene dos tiempos; repetido en el mismo fichero, comparte uno.
- Solo se escribe cuando un comando termina bien. Un fallo o un Ctrl+C no guardan nada ni borran el tiempo anterior.
- Nunca se vacía entero: cada comando sustituye su tiempo, y al guardarlo se olvidan los comandos de ese fichero que ya no están en él. Tras una ejecución completa, todos los comandos actuales tienen el de esa ejecución. Para empezar de cero, se borra `times.json`.
- Los totales se calculan recorriendo los ficheros de pasos actuales y buscando cada comando en su fichero: uno nuevo o cambiado cuenta como "sin tiempo". Añadir o mover líneas no pierde nada; renombrar un fichero pierde sus tiempos.
- **Lo que queda** desde un comando: la suma de los tiempos de ese comando y de todos los siguientes, con `, N sin tiempo` si falta alguno. Si no se conoce ninguno, no se muestra.
- **Tiempo de un fichero** y **total**: la suma de sus comandos en `times.json`, así que incluyen lo hecho antes de una reanudación.
- **Formato:** `HH:MM:SS` en todo (`00:45:10`, `01:05:30`); lo estimado lleva `~`.
- Si `times.json` falta, empieza vacío; si está corrupto, tiene el formato antiguo (comando → segundos, de la 0.2.0) o no se puede leer, se avisa y empieza vacío. Si no se puede guardar, se avisa y se sigue: los tiempos nunca detienen la ejecución.
- Se guarda de forma atómica, como `state.json`.

## 7. Flujo interactivo

### 7.1 Inicio **[cambio]**

Tras resolver las rutas (§2), todo pasa en la TUI (D12). En Python era una pregunta de texto
(`[Y/S] Continuar | [M] Menú | [O] Una vez | [N] Salir`) y, con `M`, menús curses.

- **Estado válido:** se abre la TUI con el cursor sobre la línea guardada (panel de líneas), sin popup: ya se ve
  dónde está. `Enter` abre el **popup de confirmación** (el equivalente al resumen de Python).
- **Estado inválido:** se abre la TUI navegando, con el aviso `⚠ estado no válido: elige una línea` en la cabecera.
- **Nunca se ejecuta nada sin pasar por el popup.** Salir sin elegir termina sin ejecutar nada.
- **Flags para saltarse la TUI** (opcionales): `-y/--yes` continúa desde la posición guardada y `-o/--once` ejecuta
  solo su comando. Son incompatibles entre sí. Con el estado no válido: `❌ Error: el estado no apunta a ninguna línea
  válida: lanza bash-resume sin -y/-o para elegirla` y código 1. No necesitan terminal (sirven por `ssh` sin `-t` o en scripts).
- **Sin terminal y sin flags** (stdin o stdout no son una terminal: tubería, salida redirigida, `ssh` sin `-t`):
  `❌ Error: bash-resume necesita una terminal interactiva (o usa -y/-o)` y código 1.
- Si se cierra la entrada (Ctrl+D) en una pregunta de rutas: `❌ Error: entrada cerrada` y código 1.
- Los errores de lectura o escritura de ficheros indican la ruta (por ejemplo `…/steps/02-x: stream did not contain valid UTF-8`).
- **Código de salida:** 1 si un comando falló (o por un error de E/S); 0 en el resto de casos (terminado, parada `#.`, una vez, salir).

### 7.2 Menú y confirmación (TUI) **[cambio]**

```
 Workdir: /opt/odoo/doodba-20  ·  Pasos: ~/migr/steps  ·  ● estado válido  ·  🕒 Total ~02:10:00
┌ Ficheros ─────┬ 02-migrar ──────────────────────────────────────────┐
│   01-prep     │  [001] #> MIGRANDO                                  │
│ > ● 02-migr ┌─ ¿Ejecutar? ────────────────────────────────────────┐ │
│   03-limp   │ Fichero  02-migrar · línea 3                        │ │
│             │ ▶ docker compose run --rm odoo --update all …       │ │
│             │ Workdir  /opt/odoo/doodba-20                        │ │
│             │ Pasos    ~/migr/steps                               │ │
│             │ Tiempo   00:45:10 la última vez · quedan ~01:05:00  │ │
│             │                                                     │ │
│             │ [Enter] continuar  [o] una vez  [Esc] elegir otra línea  [q] salir │
└─────────────└─────────────────────────────────────────────────────┘─┘
 02-migrar:3 → docker compose run … │ ↑↓/jk mover · Tab panel · Enter elegir · q salir
```

- **Cabecera:** directorio de trabajo, carpeta de pasos y `● estado válido` (verde) o `⚠ estado no válido: elige una línea` (amarillo). Con tiempos guardados, `🕒 Total ~02:10:00` en azul claro (§6.1).
- **Panel izquierdo:** ficheros. El del estado guardado va marcado con `●`. Si se conoce alguno de sus comandos, su tiempo en azul claro, alineado a la derecha del panel (`🕒 01:05:30`), con un pequeño margen hasta el borde.
- **Panel derecho:** líneas del fichero seleccionado, con preview en vivo, número `[001]` y color por tipo (`log` verde, `stop` rojo, `comment` gris). La línea del estado guardado sale en amarillo. A la derecha de cada comando con tiempo guardado, alineado igual que en ficheros y en azul claro, lo que tardó la última vez (`🕒 00:45:10`); si el comando es muy largo, va justo detrás.
- **Barra inferior:** `fichero:línea → comando` y la ayuda de teclas.
- **Popup de confirmación** (modal, centrado): fichero y línea, comando destacado, directorio de trabajo, carpeta de pasos, tiempos (lo que tardó ese comando la última vez y lo que queda desde él, si se sabe algo) y teclas. Mientras está abierto, solo responde a sus teclas.
- Al abrirse, con estado válido: sin popup, foco en el panel de líneas y cursor en la línea guardada. Sin estado válido: foco en ficheros, con el del estado seleccionado si existe (como en Python).
- Los ficheros se cargan todos al abrir el menú.

**Navegando**

| Tecla | Acción |
|---|---|
| `↑ ↓` / `k j` | mover (cíclico); al cambiar de fichero, el cursor de líneas vuelve a su línea guardada o a la primera |
| `Enter` / `→` / `Tab` en ficheros | pasar al panel de líneas (no en un fichero vacío) |
| `Esc` / `←` / `Tab` en líneas | volver al panel de ficheros |
| `Enter` / `y` / `s` / `o` en líneas | **abrir el popup** de confirmación con esa línea (no ejecuta) |
| `q` (en cualquier panel) / `Esc` en ficheros | salir sin ejecutar |

**Popup de confirmación**

| Tecla | Acción |
|---|---|
| `Enter` / `y` / `s` | guardar la línea en el estado y **ejecutar** desde ahí |
| `o` | guardar la línea y ejecutar **una vez** |
| `Esc` / `m` / `n` | cerrar el popup y volver a navegar |
| `q` | salir sin ejecutar |

- **[cambio]** No se puede elegir una línea de un fichero vacío. En Python esto provocaba un crash (`idx=None`).
- Ctrl+C dentro de la TUI equivale a `q` (en modo raw no llega como señal, sino como tecla).
- La TUI usa la pantalla alternativa de la terminal: al cerrarla, la terminal queda como estaba.

## 8. Log (`migration.log` en el proyecto)

- **Fichero** (en modo append), sin colores: `YYYY-mm-dd HH:MM:SS - [fichero]   LEVEL   mensaje`. `[fichero]` se rellena hasta 12 caracteres (cabe `[18-copy-db]`) y el nivel, sin dos puntos, hasta 7 (`WARNING`), con un espacio tras cada uno: así los mensajes quedan en columna con los nombres habituales; uno más largo solo desplaza su línea. Sin fichero, el hueco se rellena igual. **[cambio]** En Python, `LEVEL   : mensaje` (nivel rellenado a 8).
- **Consola (stderr):** el mismo formato, con colores ANSI (fecha azul; INFO verde, WARNING amarillo, ERROR rojo).
  **[cambio]** Solo hay colores si stderr es una terminal; redirigido a un fichero o una tubería, sale texto limpio.
- Un error al escribir el log se ignora: el log nunca detiene la ejecución.
- `[fichero]` es el fichero de pasos que se está procesando (u `onerror`). Si no hay ninguno, no se escribe.
- La salida de los comandos **no** se escribe en el log: va solo a la terminal (§4). El log solo tiene los mensajes de bash-resume.
- Cada comando ocupa dos líneas, la de inicio y la de resultado, y cada una se entiende sola (sirven para `grep`):
  ```
  … [02-migrar]   INFO    ⏳ [ 9/40] --:--:-- docker compose run --rm odoo --update all   (quedan ~01:20:00)
  … [02-migrar]   INFO    ✅ [ 9/40] 00:47:02 docker compose run --rm odoo --update all
  … [02-migrar]   ERROR   ❌ [ 9/40] 00:00:51 docker compose run --rm odoo --update all   (exit 1)
  … [02-migrar]   WARNING ⛔ [ 9/40] 00:00:03 docker compose run --rm odoo --update all   (Ctrl+C)
  ```
  El comando queda siempre en la misma columna: el número de paso se rellena por la izquierda al ancho del total
  (`[ 9/40]`) y la línea `⏳` lleva `--:--:--` donde la de resultado lleva la duración.
  Entre paréntesis, el motivo del fallo: el código de salida, la señal si murió por una o `no se pudo lanzar: <error>`.
- Otros mensajes: `#>` tal cual, `🕒 Fichero …`,
  `🕒 Resumen de tiempos:` (con una fila por fichero y `Total`) y `🕒 Esta ejecución: …` (§6.1).

## 9. CLI

```
bash-resume [WORKDIR] [-s|--steps DIR] [-p|--project DIR] [-y|--yes | -o|--once]
```

- **[cambio]** Nombres más cortos que en Python (`execution_path` → `WORKDIR`, `--steps-path` → `--steps`). Las claves de `state.json` no cambian (D4).
- Lo normal es lanzarlo **desde la carpeta del proyecto** sin argumentos (`cd proyecto && bash-resume`); `-p` sirve para lanzarlo desde otro sitio.
- `--help`/`-h` y `--version`/`-V` los genera clap. Un argumento desconocido muestra un error con una sugerencia y sale con código 2.

- Si se pulsa Ctrl+C fuera de un comando (en las preguntas), bash-resume termina sin ejecutar nada. **[cambio]** Termina por la señal (el shell muestra código 130), sin el mensaje `❌ Interrumpido por el usuario.` de Python.

## Fuera de alcance y mejoras posibles

Las ideas para el futuro (`cd` persistente, más hooks, historial de ejecución…)
están en [ideas.md](ideas.md).
