#!/usr/bin/env bash
#
# Monta dist/FalcaoTokenRouter-<versão>.dmg a partir do .app já empacotado.
#
# Usa só o que vem no macOS — `hdiutil`, `osascript`, `tiffutil`, `SetFile` —
# nada de `create-dmg` do Homebrew.
#
# ATENÇÃO: o DMG melhora a instalação, não resolve o Gatekeeper. O app continua
# assinado ad-hoc, então a primeira abertura ainda exige o passo documentado no
# README. Notarizar é o que remove aquele diálogo, e exige conta paga da Apple.
#
# Uso: ./Scripts/dmg.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/dist"
APP="$DIST/FalcaoTokenRouter.app"
VOLUME="Falcão Token Router"

[ -d "$APP" ] || { echo "erro: $APP não existe — rode ./Scripts/bundle.sh antes" >&2; exit 1; }

VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"
DMG="$DIST/FalcaoTokenRouter-$VERSION.dmg"
STAGE="$(mktemp -d)"
RAW="$(mktemp -u)-rw.dmg"
trap 'rm -rf "$STAGE" "$RAW"' EXIT

# A diagramação da janela precisa bater com a arte: a seta do fundo é desenhada
# entre estas mesmas coordenadas, em Scripts/icon.swift. Mudar uma sem a outra
# deixa a seta apontando para o lugar errado.
APP_X=172
FOLDER_X=468
ICON_Y=214
WINDOW_W=640
WINDOW_H=396
# A janela fica de propósito um pouco mais alta que a arte: o `bounds` do Finder
# inclui a barra de título, cuja altura varia entre versões do macOS. Sobrando,
# o fundo é cortado alguns pixels embaixo — onde não há nada. Faltando,
# apareceria uma faixa vazia.
WINDOW_CHROME=20

echo "==> Preparando o conteúdo"
[ -f "$DIST/AppIcon.icns" ] || "$ROOT/Scripts/icon.sh" >/dev/null

# `ditto` em vez de `cp -R`: preserva symlinks internos do bundle e os metadados
# que a assinatura cobre. `cp -R` pode invalidar a assinatura.
ditto "$APP" "$STAGE/FalcaoTokenRouter.app"
ln -s /Applications "$STAGE/Applications"

mkdir -p "$STAGE/.background"
tiffutil -cathidpicheck \
    "$DIST/installer-background.png" \
    "$DIST/installer-background@2x.png" \
    -out "$STAGE/.background/background.tiff" >/dev/null
# O TIFF multi-resolução é o que dá fundo nítido em tela retina: o Finder escolhe
# a representação pela escala, em vez de esticar a de 1x.

echo "==> Criando a imagem gravável"
SIZE_KB=$(( $(du -sk "$STAGE" | cut -f1) + 20000 ))
hdiutil create -volname "$VOLUME" -srcfolder "$STAGE" -fs HFS+ \
    -format UDRW -size "${SIZE_KB}k" -quiet "$RAW"

MOUNT="$(hdiutil attach "$RAW" -readwrite -noverify -noautoopen | \
    grep -o '/Volumes/.*$' | tail -1)"
[ -d "$MOUNT" ] || { echo "erro: não consegui montar a imagem gravável" >&2; exit 1; }

echo "==> Diagramando a janela"
# Este é o único passo que precisa de permissão de Automação: na primeira vez o
# macOS pergunta se o terminal pode controlar o Finder. Negar não quebra o DMG —
# só entrega um sem diagramação, e o aviso abaixo diz isso em vez de fingir que
# deu certo.
if osascript <<APPLESCRIPT
tell application "Finder"
    tell disk "$VOLUME"
        open
        set current view of container window to icon view
        set toolbar visible of container window to false
        set statusbar visible of container window to false
        set the bounds of container window to {200, 140, $((200 + WINDOW_W)), $((140 + WINDOW_H + WINDOW_CHROME))}
        set options to the icon view options of container window
        set arrangement of options to not arranged
        set icon size of options to 128
        set text size of options to 12
        set background picture of options to file ".background:background.tiff"
        set position of item "FalcaoTokenRouter.app" of container window to {$APP_X, $ICON_Y}
        set position of item "Applications" of container window to {$FOLDER_X, $ICON_Y}
        update without registering applications
        close
    end tell
end tell
APPLESCRIPT
then
    echo "    janela diagramada"
else
    echo "    AVISO: o Finder não pôde ser controlado — o DMG sai sem diagramação." >&2
    echo "    O app instala normalmente; só a janela fica com o visual padrão." >&2
    echo "    Para corrigir: Ajustes do Sistema → Privacidade e Segurança →" >&2
    echo "    Automação, e permita que o terminal controle o Finder." >&2
fi

# O ícone do volume entra por último, com o Finder já fora do volume.
#
# Duas armadilhas aqui, ambas silenciosas. O `hdiutil create -srcfolder` não
# leva o .VolumeIcon.icns do staging junto. E se o arquivo estiver no volume
# enquanto o Finder o diagrama, o Finder o apaga — inclusive com o atributo de
# ícone próprio já marcado. Copiar depois é o que faz sobreviver.
cp "$DIST/AppIcon.icns" "$MOUNT/.VolumeIcon.icns"
SetFile -a C "$MOUNT" || echo "    AVISO: não consegui marcar o ícone do volume" >&2

sync
hdiutil detach "$MOUNT" -quiet

echo "==> Comprimindo"
rm -f "$DMG"
hdiutil convert "$RAW" -format UDZO -imagekey zlib-level=9 -quiet -o "$DMG"

# Confere que a assinatura sobreviveu ao empacotamento — um DMG que entrega um
# app com assinatura quebrada é pior que nenhum DMG: o macOS recusa de vez, e
# não só com o aviso de quarentena.
VERIFY="$(mktemp -d)"
hdiutil attach "$DMG" -mountpoint "$VERIFY" -nobrowse -quiet
if codesign -v "$VERIFY/FalcaoTokenRouter.app" 2>/dev/null; then
    echo "==> assinatura íntegra dentro do DMG"
    hdiutil detach "$VERIFY" -quiet
    rmdir "$VERIFY" 2>/dev/null || true
else
    echo "erro: assinatura quebrada dentro do DMG" >&2
    hdiutil detach "$VERIFY" -quiet
    exit 1
fi

echo "==> Done: $DMG ($(du -h "$DMG" | cut -f1))"
