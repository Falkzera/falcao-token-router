#!/usr/bin/env bash
#
# Monta dist/FalcaoTokenRouter.app a partir do binário do SwiftPM.
# Não usa Xcode: o bundle é montado à mão e assinado ad-hoc.

set -euo pipefail

APP_NAME="FalcaoTokenRouter"
BUNDLE_ID="com.synqo.falcao-token-router"
# Fonte única da versão. Com ela fixa aqui, uma tag macos-v1.0.1 podia publicar um app
# que se diz 1.0.0 — e o único lugar onde isso apareceria seria em "Sobre" na
# máquina de quem instalou.
VERSION="$(tr -d '[:space:]' < "$(dirname "${BASH_SOURCE[0]}")/../VERSION")"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/dist"
APP="$DIST/$APP_NAME.app"

# Universal em vez de nativo: o macOS 26 ainda roda em alguns Macs Intel, e um
# binário só arm64 não abre neles — sem mensagem que explique o porquê. Custa
# uns 45s a mais de build. As duas fatias saem com minos 26.0; o aviso de
# "x86_64 deprecated" que o toolchain emite é sobre o alvo dele, não sobre o
# binário gerado.
ARCHS=(--arch arm64 --arch x86_64)
FLAVOR="universal"

# `--native` monta só para esta máquina. O build universal precisa do xcbuild,
# que só vem com o Xcode completo — sem ele, quem tem apenas Command Line Tools
# não conseguia montar nem para si mesmo. O modo nativo é explícito, e nunca o
# padrão: um artefato só arm64 publicado como release não abre em Mac Intel, e
# o sintoma seria silêncio.
for arg in "$@"; do
    if [ "$arg" = "--native" ]; then
        ARCHS=()
        FLAVOR="nativo ($(uname -m)) — NÃO publicável"
    fi
done

echo "==> Building release binary ($FLAVOR)"
swift build -c release --package-path "$ROOT" ${ARCHS[@]+"${ARCHS[@]}"}
BINDIR="$(swift build -c release --package-path "$ROOT" ${ARCHS[@]+"${ARCHS[@]}"} --show-bin-path)"
BIN="$BINDIR/$APP_NAME"
# A CLI `router` viaja dentro do app: é o sensor da status line e o launcher
# `claude <grupo>`. O app aponta a statusLine e a função de shell para ela.
ROUTER_BIN="$BINDIR/router"

echo "==> Checking strings"
"$ROOT/Scripts/check-strings.sh" >/dev/null

echo "==> Generating icon"
"$ROOT/Scripts/icon.sh" >/dev/null

echo "==> Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/$APP_NAME"
cp "$ROUTER_BIN" "$APP/Contents/MacOS/router"
cp "$DIST/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"

# Os .lproj vão para dentro do bundle montado à mão. Ficam fora de Sources/ de
# propósito: o SwiftPM reclama de arquivo solto num target que ele não sabe
# tratar, e aqui não há target que os declare — quem monta o bundle é este
# script.
cp -R "$ROOT"/Resources/*.lproj "$APP/Contents/Resources/"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>$APP_NAME</string>
    <key>CFBundleDisplayName</key><string>Falcão Token Router</string>
    <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
    <key>CFBundleExecutable</key><string>$APP_NAME</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <!-- Inglês como base: quem estiver num idioma sem tradução recebe inglês,
         não português. O fallback tem que ser o que mais gente consegue ler. -->
    <key>CFBundleDevelopmentRegion</key><string>en</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundleVersion</key><string>$VERSION</string>
    <key>LSMinimumSystemVersion</key><string>26.0</string>
    <!-- Agent app: vive só na menu bar, sem ícone no Dock. -->
    <key>LSUIElement</key><true/>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Se uma das fatias sumir, o app deixa de abrir em metade dos Macs e nada aqui
# reclamaria — a checagem existe para transformar isso em erro de build. Em
# `--native` a ausência é intencional, então checar seria transformar a escolha
# do usuário em erro.
if [ "${#ARCHS[@]}" -gt 0 ]; then
    for arch in arm64 x86_64; do
        lipo -archs "$APP/Contents/MacOS/$APP_NAME" | grep -qw "$arch" \
            || { echo "erro: binário sem a fatia $arch" >&2; exit 1; }
    done
fi

echo "==> Ad-hoc signing"
codesign --force --deep --sign - "$APP"

echo "==> Done: $APP"

# `--install` copia para /Applications. O launch at login precisa disso: o
# SMAppService registra um caminho, e um app que vive em dist/ tem o binário
# trocado a cada rebuild — o login item passaria a apontar para lixo.
if [[ " $* " == *" --install "* ]]; then
    TARGET="/Applications/$APP_NAME.app"
    echo "==> Installing to $TARGET"
    rm -rf "$TARGET"
    cp -R "$APP" "$TARGET"
    codesign --force --deep --sign - "$TARGET"
    echo "==> Installed: $TARGET"
fi
