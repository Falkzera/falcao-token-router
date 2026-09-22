#!/usr/bin/env bash
#
# Confere que as chaves usadas no código existem em todos os catálogos, e que
# nenhum catálogo carrega tradução órfã.
#
# Existe porque este é o tipo de defeito que não produz erro nenhum: acrescentar
# uma string no código sem traduzir compila, passa nos testes, e mostra a chave
# crua para o usuário. O compilador não tem como saber.
#
# Uso: ./Scripts/check-strings.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Os prefixos são o contrato: chave de usuário começa por um destes. Restringir
# assim evita confundir com identificador de SF Symbol ou URL de esquema.
PREFIXES='panel|settings|alerts|format|groups|home'
FAILED=0

USED="$(grep -rhoE "\"($PREFIXES)\.[a-zA-Z.]+\"" "$ROOT/Sources" \
        | tr -d '"' | sort -u)"

for catalog in "$ROOT"/Resources/*.lproj/Localizable.strings; do
    locale="$(basename "$(dirname "$catalog")" .lproj)"
    DEFINED="$(grep -oE '^"[^"]+"' "$catalog" | tr -d '"' | sort -u)"

    if MISSING="$(comm -23 <(echo "$USED") <(echo "$DEFINED"))" && [ -n "$MISSING" ]; then
        echo "erro: chaves usadas no código e ausentes em $locale:" >&2
        echo "$MISSING" | sed 's/^/    /' >&2
        FAILED=1
    fi

    if ORPHAN="$(comm -13 <(echo "$USED") <(echo "$DEFINED"))" && [ -n "$ORPHAN" ]; then
        echo "erro: traduções órfãs em $locale (nenhum código as usa):" >&2
        echo "$ORPHAN" | sed 's/^/    /' >&2
        FAILED=1
    fi
done

# Terceira checagem, e a que pegou o caso real: literal cru numa view. As duas
# acima comparam chave com catálogo, e nada nelas nota uma string que nunca
# virou chave. `Text(verbatim:)` é a saída explícita para o que não se traduz.
LOOSE="$(grep -rnoE '(Text|Label|Button|Toggle|Section|Picker|LabeledContent|TextField)\("[^"]+"' \
         "$ROOT/Sources" | grep -vE "\"($PREFIXES)\." || true)"
if [ -n "$LOOSE" ]; then
    echo "erro: literal em view fora do catálogo (use uma chave, ou Text(verbatim:)):" >&2
    echo "$LOOSE" | sed 's/^/    /' >&2
    FAILED=1
fi

[ "$FAILED" -eq 0 ] || exit 1
echo "==> strings ok: $(echo "$USED" | wc -l | tr -d ' ') chaves em $(ls -d "$ROOT"/Resources/*.lproj | wc -l | tr -d ' ') idiomas"
