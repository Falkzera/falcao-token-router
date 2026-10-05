#!/usr/bin/env bash
#
# Gera dist/AppIcon.icns, a arte do instalador, o banner e o card social.
#
# A arte é da marca Falcão: o símbolo e o nome saem dos SVGs oficiais, copiados
# no icon.swift, e as fontes da marca ficam em Scripts/fonts/. O anel é o
# medidor, e quem o desenha é a barra de menus — não este script.
#
# Uso: ./Scripts/icon.sh [captura-crua-do-painel.png]
#
# Passando uma captura, ela também é emoldurada em dist/panel.png para o README.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# A arte entra no repositório, não no projeto macOS: o banner e o card social
# são do produto inteiro, e o README que os aponta mora na raiz de cima.
REPO="$(cd "$ROOT/.." && pwd)"
OUT="$ROOT/dist"
BUILD="$(mktemp -d)"
trap 'rm -rf "$BUILD"' EXIT

mkdir -p "$OUT"

echo "==> Compilando o gerador"
swiftc -O -parse-as-library "$ROOT/Scripts/icon.swift" -o "$BUILD/icongen"

echo "==> Desenhando"
"$BUILD/icongen" "$OUT" ${1:+"$1"}

echo "==> Montando o .icns"
iconutil --convert icns "$OUT/AppIcon.iconset" --output "$OUT/AppIcon.icns"

# Banner e card social entram no repo: o README aponta para o banner, e o card
# social é enviado ao GitHub à mão nas configurações. dist/ é ignorado, então
# ficar só lá significaria README quebrado para quem clona.
#
# Só quando este script é chamado de propósito. O `bundle.sh` o roda em todo
# build (precisa do .icns) e passa `ICON_SKIP_REPO_ART=1`: o texto da arte é
# rasterizado pelo sistema, e um macOS de outra versão desenha pixels diferentes
# — cada build sujava `docs/art/`, e o `release.sh` recusa árvore suja.
if [ -z "${ICON_SKIP_REPO_ART:-}" ]; then
    mkdir -p "$REPO/docs/art"
    cp "$OUT/banner.png" "$OUT/social-preview.png" "$REPO/docs/art/"
    if [ -f "$OUT/panel.png" ]; then cp "$OUT/panel.png" "$REPO/docs/art/"; fi
fi

echo "==> Done: $OUT/AppIcon.icns ($(du -h "$OUT/AppIcon.icns" | cut -f1))"
