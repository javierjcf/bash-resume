#!/bin/sh
# Instala bash-resume desde las releases de GitHub.
#
#   curl -fsSL https://raw.githubusercontent.com/javierjcf/bash-resume/main/install.sh | sh
#
# Variables opcionales:
#   BASH_RESUME_VERSION      versión a instalar, por ejemplo v0.1.0 (por defecto, la última release)
#   BASH_RESUME_INSTALL_DIR  carpeta de destino (por defecto, ~/.local/bin; para todos los usuarios,
#                         /usr/local/bin con: ... | sudo env BASH_RESUME_INSTALL_DIR=/usr/local/bin sh)
#
# Escrito en sh POSIX (no bash): con `curl | sh` lo ejecuta /bin/sh, que en Debian/Ubuntu es dash.
set -eu

REPO="javierjcf/bash-resume"
TARGET="x86_64-unknown-linux-musl"
INSTALL_DIR="${BASH_RESUME_INSTALL_DIR:-$HOME/.local/bin}"

fail() {
    echo "❌ $*" >&2
    exit 1
}

# El binario publicado es para Linux x86_64 (estático, funciona en cualquier distribución).
[ "$(uname -s)" = "Linux" ] || fail "solo hay binario para Linux (este sistema es $(uname -s))"
[ "$(uname -m)" = "x86_64" ] || fail "solo hay binario para x86_64 (este sistema es $(uname -m))"
command -v curl >/dev/null || fail "hace falta curl"
command -v tar >/dev/null || fail "hace falta tar"

# Versión: la indicada o la última release (GitHub redirige /releases/latest a /releases/tag/vX.Y.Z).
VERSION="${BASH_RESUME_VERSION:-}"
if [ -z "$VERSION" ]; then
    latest_url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest") ||
        fail "no se encontró ninguna release publicada en https://github.com/$REPO/releases"
    VERSION="${latest_url##*/}"
fi
case "$VERSION" in
    v*) ;;
    *) fail "no se encontró ninguna release publicada en https://github.com/$REPO/releases" ;;
esac

NAME="bash-resume-$VERSION-$TARGET"
URL="https://github.com/$REPO/releases/download/$VERSION/$NAME.tar.gz"

# Descarga en una carpeta temporal que se borra al terminar, pase lo que pase.
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "⬇️  Descargando bash-resume $VERSION…"
curl -fsSL -o "$TMP/$NAME.tar.gz" "$URL" || fail "no se pudo descargar $URL"

# Comprobar el checksum, si se puede (la release publica el .sha256 junto al .tar.gz).
if command -v sha256sum >/dev/null && curl -fsSL -o "$TMP/$NAME.tar.gz.sha256" "$URL.sha256"; then
    (cd "$TMP" && sha256sum -c --quiet "$NAME.tar.gz.sha256") || fail "el checksum no coincide: descarga corrupta"
else
    echo "⚠️  No se ha podido comprobar el checksum (falta sha256sum o el fichero .sha256)"
fi

tar -xzf "$TMP/$NAME.tar.gz" -C "$TMP"
mkdir -p "$INSTALL_DIR"
install -m 755 "$TMP/$NAME/bash-resume" "$INSTALL_DIR/bash-resume"

echo "✅ Instalado: $INSTALL_DIR/bash-resume ($("$INSTALL_DIR/bash-resume" --version))"

# Avisar si la carpeta no está en el PATH.
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo "⚠️  $INSTALL_DIR no está en tu PATH. Añádelo con:"
        echo "    echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc   # o ~/.zshrc"
        ;;
esac
