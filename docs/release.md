# Publicar una release

Una release son dos mitades (D18):

| Quién | Qué | Cómo |
|---|---|---|
| **Tú** | Decidir la versión y revisar el CHANGELOG | `scripts/release.sh X.Y.Z` |
| **GitHub Actions** | Comprobar, compilar (musl), empaquetar y publicar | Solo, al recibir el tag `vX.Y.Z` |

## Chuleta

```bash
# 1. Repasar lo que se va a publicar: la sección [Unreleased] de CHANGELOG.md.
#    Se va llenando al cerrar cada tarea; si falta algo, se añade y se hace commit.

# 2. Lanzar la release (en main y con el árbol limpio):
scripts/release.sh 0.2.1

# 3. Seguir la publicación (opcional): https://github.com/javierjcf/bash-resume/actions
gh run watch $(gh run list -w Release -L 1 --json databaseId -q '.[0].databaseId')

# 4. Actualizar el binario propio:
curl -fsSL https://raw.githubusercontent.com/javierjcf/bash-resume/main/install.sh | sh
```

**Qué versión** ([SemVer](https://semver.org/lang/es/), estando en 0.x): `0.2.1` para correcciones y cambios
pequeños; `0.3.0` si hay algo nuevo o algo cambia de forma incompatible (la CLI, el formato de `state.json`…).

## Qué hace `scripts/release.sh`

Acepta `0.2.1` o `v0.2.1`; el tag siempre es `v0.2.1` y en `Cargo.toml` va `0.2.1`.

1. **Comprueba**: formato `X.Y.Z`, rama `main`, árbol limpio, no por detrás de `origin/main` y que el tag no existe.
   Si hay `cargo`, también fmt, clippy y tests.
2. **Edita** con `sed`:
   - `Cargo.toml` y `Cargo.lock`: la versión (no hace falta cargo).
   - `CHANGELOG.md`: solo lo mecánico. `## [Unreleased]` pasa a `## [0.2.1] - fecha` con un `[Unreleased]` vacío
     encima, y el enlace `[Unreleased]` del final se divide en dos. El **contenido** se escribe a mano antes.
3. **Enseña** el diff y los commits que entran, y pregunta. Si dices que no, deja los ficheros como estaban.
4. **Publica**: commit `chore(release): v0.2.1`, tag anotado y `git push --atomic origin main v0.2.1` (suben los dos
   o ninguno). Sube también los commits de `main` que aún no estuvieran en GitHub.

**Qué dispara la publicación:** solo subir un tag `vX.Y.Z`. Un push normal a `main` no publica nada.

## Qué hace GitHub Actions (`.github/workflows/release.yml`)

Al recibir un tag `vX.Y.Z`:

1. Comprueba que el tag coincide con `Cargo.toml` y que el CHANGELOG tiene la sección `[X.Y.Z]`.
2. `cargo fmt --check`, `clippy -D warnings` y `cargo test`, con `--locked`.
3. Compila `x86_64-unknown-linux-musl` y comprueba que el binario es estático.
4. Empaqueta `bash-resume-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` (binario, README y CHANGELOG) con su `.sha256`.
5. Crea la release **publicada** (no borrador), con la sección del CHANGELOG como notas.
6. En otra máquina, instala con `install.sh` y comprueba que `bash-resume --version` es la nueva.

Si un paso falla, no se crea la release (o, si falla el 6, queda publicada pero el job sale en rojo).

## Si algo sale mal

**Falla el workflow antes de publicar** (tests, clippy, versión…): el commit y el tag ya están en GitHub, pero no hay
release. Se arregla, se hace commit y se mueve el tag a ese commit:

```bash
V=v0.2.1
# arreglar, git add <ficheros> && git commit -m "fix(...): ..."
git tag -d $V && git push origin --delete $V
git tag -a $V -m "$V" && git push origin main $V      # vuelve a lanzar el workflow
```

**Reintentar sin cambios** (un fallo de red de GitHub): en la pestaña *Actions*, abrir el run y *Re-run jobs*.

**Ya está publicada y tiene un error**: no se rehace; se publica la siguiente (`0.2.2`). Solo si nadie la ha
descargado aún: `gh release delete $V --yes` y rehacer el tag como arriba.

## Trampas conocidas

- **Versión y tag deben coincidir** (`version = "0.2.1"` ⇔ `v0.2.1`): el script los pone igual y el workflow lo comprueba.
- **`install.sh` usa `/releases/latest`**: solo ve releases publicadas, no borradores.
- **Paginador**: `git log` / `git diff` abren `less` → `q` para salir, o `git --no-pager …`.
- **Publicar a mano** (sin Actions) sigue siendo posible: son los pasos del workflow, en el mismo orden.
