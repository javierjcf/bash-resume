# Especificación funcional — stepwise

Qué hace el programa, visto desde fuera. No describe cómo está hecho en Rust: eso va en
[decisions.md](decisions.md) y en el código. La referencia es la versión Python en `pyBashResume/`.
Las diferencias deliberadas están marcadas con **[cambio]**.

## 1. Concepto

`stepwise` ejecuta, en orden, las líneas de una serie de ficheros de pasos. Cada línea es un
comando de shell independiente. Recuerda por dónde va y permite reanudar desde el último fallo,
o desde cualquier punto elegido en un menú interactivo.

## 2. Directorios

| Nombre | Contenido | Cómo se resuelve |
|---|---|---|
| **home** | `state.json`, `migration.log`, `onerror` | `-H/--home DIR` → env `STEPWISE_HOME` → pregunta interactiva. No se guarda en el estado. **[cambio]** En Python era el directorio del script. |
| **workdir** (`execution_path` en el estado) | Directorio donde se ejecutan los comandos | argumento posicional `WORKDIR` → `state.json` → pregunta interactiva |
| **steps** (`steps_path` en el estado) | Carpeta con los ficheros de pasos | `-s/--steps` → `state.json` → `migration-steps` → pregunta interactiva |

- **[cambio]** En Python, `--steps-path` tenía `default="migration-steps"` en argparse, así que si existía `./migration-steps`, siempre ganaba al valor de `state.json`. Ahora el valor por defecto solo se usa si no hay argumento ni estado.
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

- Se ejecuta como `sh -c "<línea>"` en `execution_path`. Cada línea es un proceso nuevo, así que un `cd` no afecta a la siguiente.
- **[cambio]** No se hace `chdir` en el propio programa: cada comando se lanza con ese directorio de trabajo (D8). El comando ve lo mismo.
- Si el comando no se puede lanzar (por ejemplo, el directorio no existe), cuenta como fallo.
- El comando **hereda la terminal**: su salida aparece en tiempo real con sus colores y barras de progreso, y puede leer del teclado (prompts de `sudo`, `[y/N]`…). **[cambio]** En Python se usaba un PTY y los prompts no funcionaban.
- **¿Es un error?** `exit 0` → éxito; cualquier otro código (o morir por una señal) → fallo.
  **[cambio]** Se elimina la heurística de Python para `exit 255` + `--stop-after-init`: en Odoo un 255 es siempre un error real (D9). Para aceptar un código en un paso concreto: `cmd || true`.
- **Ctrl+C** durante un comando: la terminal se lo envía al comando, que se detiene; stepwise **no** se detiene, registra `⛔ Ctrl+C detectado…` y lo trata como un fallo (estado en esa línea + `onerror`, código 1), igual que Python. Cuenta como fallo aunque el comando termine con 0 al recibirlo.
- Al terminar se registra en el log `Tiempo de ejecución: h:m:s` (sin ceros a la izquierda).

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

### onerror

- Es el fichero `onerror` dentro de **home**. Si no existe → WARNING en el log.
- Se ejecuta línea a línea, saltando las vacías y las que empiezan por `#`. Sus fallos se ignoran.

## 6. Estado (`state.json` en home)

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
- Se guarda **en cada cambio**.
- **Válido** si: `current_file`, `last_line_index` y `last_command` tienen valor; el fichero existe; la línea existe, y su texto (sin espacios al inicio ni al final) coincide con `last_command`.
- **[cambio]** `last_line_index = 0` no es válido (en Python, `lines[0 - 1]` apuntaba a la última línea).
- Rust escribe el JSON con 2 espacios de sangría y Python con 4; los dos leen cualquiera.

## 7. Flujo interactivo

### 7.1 Inicio

- **Estado válido:** se muestra el resumen (directorios, fichero, línea y comando) y la pregunta
  `[Y/S] Continuar | [M] Menú | [O] Una vez | [N] Salir`.
- **Estado inválido:** `El estado previo no es válido…` y la pregunta `[M] Abrir menú | [N] Salir`, que se repite hasta recibir una respuesta válida.
- **[cambio]** Una respuesta no válida con estado válido vuelve a preguntar (en Python abría el menú).
- Tras elegir en el menú se vuelve a esta pregunta, ya con el nuevo punto (en Python era una pregunta aparte, igual).
- Si se cierra la entrada (Ctrl+D) en una pregunta: `❌ Error: entrada cerrada` y código 1.
- **Código de salida:** 1 si un comando falló (o por un error de E/S); 0 en el resto de casos (terminado, parada `#.`, una vez, salir).
- *Provisional hasta T11–T12:* `[M]` sitúa el estado en la primera línea del primer fichero con contenido.

### 7.2 Menú (TUI) **[cambio]**

En Python eran dos menús curses encadenados más una pregunta de confirmación. Ahora es una sola pantalla:

- **Panel izquierdo:** ficheros. El del estado actual va marcado con `●`.
- **Panel derecho:** líneas del fichero seleccionado, con preview en vivo, número `[001]` y color por tipo (`log` verde, `stop` rojo, `comment` gris). La línea del estado guardado sale en amarillo.
- **Barra inferior:** `fichero:línea → comando` y la ayuda de teclas.
- Al abrirse, el foco está en ficheros, con el fichero del estado seleccionado (si existe); el cursor de líneas está en la línea guardada si sigue teniendo el mismo comando, y si no en la primera (como en Python).
- Los ficheros se cargan todos al abrir el menú.

| Tecla | Acción |
|---|---|
| `↑ ↓` / `k j` | mover (cíclico); al cambiar de fichero, el cursor de líneas vuelve a su línea guardada o a la primera |
| `Enter` / `→` / `Tab` en ficheros | pasar al panel de líneas (no en un fichero vacío) |
| `Esc` / `←` / `Tab` en líneas | volver al panel de ficheros |
| `Enter` / `y` / `s` en líneas | guardar la selección en el estado y **ejecutar** |
| `o` en líneas | guardar la selección y ejecutar **una vez** |
| `q` (en cualquier panel) / `Esc` en ficheros | salir sin ejecutar |

- **[cambio]** No se puede confirmar en un fichero vacío. En Python esto provocaba un crash (`idx=None`).

## 8. Log (`migration.log` en home)

- **Fichero** (en modo append), sin colores: `YYYY-mm-dd HH:MM:SS - [fichero] LEVEL   : mensaje`. El nivel se rellena hasta 8 caracteres.
- **Consola (stderr):** el mismo formato, con colores ANSI (fecha azul; INFO verde, WARNING amarillo, ERROR rojo).
  **[cambio]** Solo hay colores si stderr es una terminal; redirigido a un fichero o una tubería, sale texto limpio.
- Un error al escribir el log se ignora: el log nunca detiene la ejecución.
- `[fichero]` es el fichero de pasos que se está procesando (u `onerror`). Si no hay ninguno, no se escribe.
- Mensajes principales: `Ejecutando comando: …`, `🔚 Proceso finalizado correctamente (exit code: 0)`,
  `⚠️ Proceso finalizado con código de salida no cero: N`, `Comando fallido (exit code N): …`,
  `✅ Comando completado correctamente (exit code N): …`.

## 9. CLI

```
stepwise [WORKDIR] [-s|--steps DIR] [-H|--home DIR]
```

- **[cambio]** Nombres más cortos que en Python (`execution_path` → `WORKDIR`, `--steps-path` → `--steps`). Las claves de `state.json` no cambian (D4).
- `--help`/`-h` y `--version`/`-V` los genera clap. Un argumento desconocido muestra un error con una sugerencia y sale con código 2.

- Si se pulsa Ctrl+C fuera de un comando (en las preguntas), stepwise termina sin ejecutar nada. **[cambio]** Termina por la señal (el shell muestra código 130), sin el mensaje `❌ Interrumpido por el usuario.` de Python.

## Fuera de alcance (por ahora) y mejoras posibles

- `cd` persistente entre líneas (por ejemplo, con una marca `#cd dir`). Encaja en el runner si se necesita.
- **Preguntar tras un Ctrl+C** en vez de dar el paso por fallado: `[R] Reintentar | [S] Saltar y seguir | [N] Salir`.
  Ctrl+C se mantiene como gesto para interrumpir: una tecla propia (por ejemplo `S`) chocaría con los prompts
  de los comandos, porque mientras corren el teclado es suyo (D9). Con `R`/`S` no se lanzaría `onerror`
  (la interrupción es intencionada). Encajaría como `Outcome::Interrupted` en el runner y una pregunta en `app`.
