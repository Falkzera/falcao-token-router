#!/usr/bin/env bash
#
# Roda a suíte do core.
#
# Command Line Tools traz o swift-testing mas não o conecta ao `swift test`:
# o plugin de macros não está no plugin path padrão e as bibliotecas de runtime
# não entram no rpath do bundle de teste. O toolchain do Xcode faz isso sozinho,
# então os flags abaixo só são adicionados quando os diretórios do CLT existem.
#
# Uso: ./Scripts/test.sh [args extras do swift test]
#   ./Scripts/test.sh --filter PricingTable

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEV="$(xcode-select -p)"

ARGS=()
PLUGINS="$DEV/usr/lib/swift/host/plugins/testing"
FRAMEWORKS="$DEV/Library/Developer/Frameworks"
INTEROP="$DEV/Library/Developer/usr/lib"

[ -d "$PLUGINS" ]    && ARGS+=(-Xswiftc -plugin-path -Xswiftc "$PLUGINS")
# O -F é para o compilador, o -rpath é para o linker, e um não substitui o
# outro. No CLT o swift-testing vem como Testing.framework: sem o search path
# de framework, `import Testing` não resolve e a suíte nem chega a linkar.
[ -d "$FRAMEWORKS" ] && ARGS+=(-Xswiftc -F -Xswiftc "$FRAMEWORKS"
                               -Xlinker -rpath -Xlinker "$FRAMEWORKS")
[ -d "$INTEROP" ]    && ARGS+=(-Xlinker -rpath -Xlinker "$INTEROP")

# Catálogos antes dos testes: chave sem tradução não quebra compilação nem teste,
# só aparece crua para o usuário. Aqui é onde isso vira erro.
"$ROOT/Scripts/check-strings.sh"

# ${ARGS[@]+...} protege contra array vazio sob `set -u` no bash 3.2 do macOS.
exec swift test --package-path "$ROOT" ${ARGS[@]+"${ARGS[@]}"} "$@"
