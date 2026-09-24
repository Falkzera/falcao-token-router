#!/usr/bin/env bash
#
# Corta uma release: confere, cria a tag e empurra. O BUILD acontece na nuvem
# (.github/workflows/release.yml), que tem o Xcode completo para o binário
# universal — esta máquina, só com Command Line Tools, não tem.
#
# Existe porque cortar release à mão é como se publica artefato velho ou tag
# fora de sincronia com VERSION. O workflow confere de novo do lado de lá.
#
# Uso: ./Scripts/release.sh [--dry-run]
#
# A versão vem do arquivo VERSION. Para subir de versão, edite aquele arquivo,
# atualize o CHANGELOG.md, mergeie na main — e aí rode isto NA main.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(tr -d '[:space:]' < "$ROOT/VERSION")"
# O prefixo distingue das tags do porte Windows (`windows-v*`).
TAG="macos-v$VERSION"
DRY_RUN=false
for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=true ;;
        *) echo "erro: argumento desconhecido: $arg" >&2; exit 1 ;;
    esac
done

echo "==> Release $TAG${DRY_RUN:+ (simulação)}"

# A main é protegida e é dela que o workflow constrói; uma tag em outra branch
# publicaria um commit que a página do projeto não mostra.
BRANCH="$(git -C "$ROOT" rev-parse --abbrev-ref HEAD)"
[ "$BRANCH" = "main" ] || { echo "erro: release sai da main, não de '$BRANCH'" >&2; exit 1; }

if [ -n "$(git -C "$ROOT" status --porcelain)" ]; then
    echo "erro: há mudanças não commitadas — a release sairia de um estado que" >&2
    echo "      ninguém consegue reproduzir depois" >&2
    exit 1
fi

git -C "$ROOT" fetch -q origin main
if [ "$(git -C "$ROOT" rev-parse HEAD)" != "$(git -C "$ROOT" rev-parse origin/main)" ]; then
    echo "erro: a main local difere da origin/main — dê pull (ou push) antes" >&2
    exit 1
fi

if git -C "$ROOT" rev-parse "$TAG" >/dev/null 2>&1 \
   || git -C "$ROOT" ls-remote --tags origin "refs/tags/$TAG" | grep -q .; then
    echo "erro: a tag $TAG já existe. Suba a versão no arquivo VERSION." >&2
    exit 1
fi

grep -q "^## \[$VERSION\]" "$ROOT/CHANGELOG.md" \
    || { echo "erro: CHANGELOG.md não tem a seção [$VERSION]" >&2; exit 1; }

# Barato, e evita empurrar uma tag que o workflow vai rejeitar dez minutos depois.
echo "==> Testes"
"$ROOT/Scripts/test.sh" >/dev/null

if $DRY_RUN; then
    echo "==> Simulação: a tag $TAG seria criada em $(git -C "$ROOT" rev-parse --short HEAD)."
    exit 0
fi

echo "==> Tag"
git -C "$ROOT" tag -a "$TAG" -m "$TAG"
git -C "$ROOT" push origin "$TAG"

echo "==> A nuvem constrói agora. Acompanhe:"
echo "    gh run watch \$(gh run list --workflow release.yml --limit 1 --json databaseId -q '.[0].databaseId')"
echo "    A release nasce como RASCUNHO — revise as notas e publique em:"
echo "    https://github.com/Falkzera/falcao-token-router/releases/tag/$TAG"
