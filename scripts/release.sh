#!/usr/bin/env bash
# Prepara y lanza una release (D18): versión, cabecera del CHANGELOG, commit, tag y push.
# Lo demás (compilar, empaquetar y publicar) lo hace GitHub Actions al recibir el tag (.github/workflows/release.yml).
#
#   scripts/release.sh 0.2.1        (o v0.2.1: la v es opcional; el tag siempre es v0.2.1)
#
# El contenido del CHANGELOG se escribe a mano en [Unreleased]; aquí solo se le pone versión y fecha.
# Antes de hacer commit enseña el diff y pregunta; si dices que no, deja los ficheros como estaban.
set -euo pipefail

REPO_URL="https://github.com/javierjcf/bash-resume"
FILES=(Cargo.toml Cargo.lock CHANGELOG.md)

fail() {
    echo "❌ $*" >&2
    exit 1
}

cd "$(git rev-parse --show-toplevel)"

# 1. Versión ---------------------------------------------------------------------------------------------------
[ $# -eq 1 ] || fail "uso: scripts/release.sh X.Y.Z   (por ejemplo, 0.2.1)"
VERSION=${1#v}
[[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "versión no válida: '$1' (formato X.Y.Z)"
TAG=v$VERSION
CURRENT=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)

# 2. Comprobaciones ----------------------------------------------------------------------------------------------
[ "$(git branch --show-current)" = "main" ] || fail "hay que estar en main"
[ -z "$(git status --porcelain)" ] || fail "hay cambios sin commit (git status)"
git fetch --quiet origin main --tags
git merge-base --is-ancestor origin/main HEAD || fail "main está por detrás de origin/main: haz git pull"
if git rev-parse --quiet --verify "refs/tags/$TAG" >/dev/null; then fail "el tag $TAG ya existe"; fi

if command -v cargo >/dev/null; then
    cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked --quiet
else
    echo "⚠️  No hay cargo: fmt, clippy y tests los pasará GitHub Actions antes de publicar."
fi

# 3. Editar ------------------------------------------------------------------------------------------------------
# Cargo.toml y Cargo.lock (en el lock, la línea version que sigue a name = "bash-resume").
sed -i "0,/^version = \"$CURRENT\"/s//version = \"$VERSION\"/" Cargo.toml
sed -i "/^name = \"bash-resume\"$/{n;s/^version = .*/version = \"$VERSION\"/}" Cargo.lock

# CHANGELOG: la cabecera [Unreleased] pasa a [X.Y.Z] - fecha (con un [Unreleased] vacío encima)
# y el enlace [Unreleased] del final se divide en dos.
sed -i "s|^## \[Unreleased\]$|## [Unreleased]\n\n## [$VERSION] - $(date +%F)|" CHANGELOG.md
sed -i "s|^\[Unreleased\]: .*/compare/v$CURRENT\.\.\.HEAD$|[Unreleased]: $REPO_URL/compare/$TAG...HEAD\n[$VERSION]: $REPO_URL/compare/v$CURRENT...$TAG|" CHANGELOG.md

# Si algún sed no encontró lo que buscaba, no se sigue.
if ! grep -q "^version = \"$VERSION\"" Cargo.toml ||
    ! grep -A1 '^name = "bash-resume"$' Cargo.lock | grep -q "^version = \"$VERSION\"" ||
    ! grep -q "^## \[$VERSION\] - " CHANGELOG.md ||
    ! grep -q "^\[$VERSION\]: " CHANGELOG.md; then
    git checkout -- "${FILES[@]}"
    fail "no se pudieron actualizar la versión o el CHANGELOG (¿formato distinto?); ficheros restaurados"
fi

# 4. Confirmar y publicar ----------------------------------------------------------------------------------------
git --no-pager diff -- Cargo.toml CHANGELOG.md
echo
echo "Commits que entran en $TAG:"
git --no-pager log --oneline "v$CURRENT..HEAD"
echo
read -r -p "¿Publicar $TAG? Se hará commit, tag y push a origin [s/N] " answer
if [[ ! $answer =~ ^[sS]$ ]]; then
    git checkout -- "${FILES[@]}"
    echo "↩️  Cancelado: ficheros restaurados, nada publicado."
    exit 0
fi

git add "${FILES[@]}"
git commit --quiet -m "chore(release): $TAG"
git tag -a "$TAG" -m "$TAG"
git push --atomic origin main "$TAG" # suben main y el tag, o ninguno

echo
echo "✅ $TAG subido. GitHub Actions compila y publica la release (unos 3 minutos):"
echo "   $REPO_URL/actions"
echo "   $REPO_URL/releases/tag/$TAG"
