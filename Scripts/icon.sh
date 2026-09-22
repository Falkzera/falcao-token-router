#!/usr/bin/env bash
#
# Gera dist/AppIcon.icns e a arte do instalador.
#
# O gerador é compilado junto com GaugeGeometry.swift, o mesmo arquivo que o app
# usa para desenhar o anel na barra de menu — é isso que mantém o ícone e a
# barra sendo o mesmo desenho, e não dois parecidos.
#
# Uso: ./Scripts/icon.sh [captura-crua-do-painel.png]
#
# Passando uma captura, ela também é emoldurada em dist/panel.png para o README.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/dist"
BUILD="$(mktemp -d)"
trap 'rm -rf "$BUILD"' EXIT

mkdir -p "$OUT"

echo "==> Compilando o gerador"
swiftc -O \
    "$ROOT/Scripts/icon.swift" \
    "$ROOT/Sources/CCUsageCore/Presentation/GaugeGeometry.swift" \
    -o "$BUILD/icongen"

echo "==> Desenhando"
"$BUILD/icongen" "$OUT" ${1:+"$1"}

echo "==> Montando o .icns"
iconutil --convert icns "$OUT/AppIcon.iconset" --output "$OUT/AppIcon.icns"

# Banner e card social entram no repo: o README aponta para o banner, e o card
# social é enviado ao GitHub à mão nas configurações. dist/ é ignorado, então
# ficar só lá significaria README quebrado para quem clona.
mkdir -p "$ROOT/docs/art"
cp "$OUT/banner.png" "$OUT/social-preview.png" "$ROOT/docs/art/"
[ -f "$OUT/panel.png" ] && cp "$OUT/panel.png" "$ROOT/docs/art/"

echo "==> Done: $OUT/AppIcon.icns ($(du -h "$OUT/AppIcon.icns" | cut -f1))"
