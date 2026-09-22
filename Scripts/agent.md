# Scripts — agent.md

## Propósito
Tudo que constrói, confere e publica — sem Xcode e sem Homebrew. Cada script diz no cabeçalho por que existe e o que NÃO resolve.

## Arquivos
- `test.sh` — a suíte. Injeta o plugin de macro do swift-testing e os rpaths que o Command Line Tools não conecta sozinho; `swift test` puro **não funciona** aqui. Roda `check-strings.sh` antes.
- `check-strings.sh` — chaves usadas no código × catálogos `en`/`pt-BR`, e literal solto em view. `PREFIXES` é o registro de namespaces: superfície nova pede prefixo novo.
- `bundle.sh` — monta `dist/FalcaoTokenRouter.app` a partir do SwiftPM e assina ad-hoc. Universal por padrão (precisa do xcbuild do Xcode); `--native` monta só para esta máquina e **não é publicável**. `--install` copia para `/Applications`.
- `dmg.sh` — o DMG, com diagramação de janela pelo Finder quando há tela; sem tela avisa e sai sem diagramação.
- `icon.sh` + `icon.swift` — o ícone e a arte, compilados junto com `GaugeGeometry.swift` para o ícone e a barra serem o MESMO desenho.
- `release.sh` — confere (main limpa e sincronizada, tag inexistente, CHANGELOG com a seção, testes) e empurra a tag. O build é da nuvem: `.github/workflows/release.yml`.

## Padrões
- `set -euo pipefail` em tudo; `${ARR[@]+"${ARR[@]}"}` para array vazio sob `set -u` no bash 3.2 do macOS.
- Nada aqui depende de ferramenta fora do macOS + Command Line Tools.

## Decisões recentes
- 2026-09-22: a release passou a ser construída na nuvem. O binário universal exige o xcbuild, que só vem com o Xcode completo — a máquina do mantenedor tem só o CLT. O `release.sh` ficou com a parte que precisa de julgamento local (está na main? está limpo? a tag existe?); a release nasce como **rascunho** para as notas serem revisadas antes de aparecer.

## Pendências conhecidas
- Assinatura ad-hoc. Notarizar exige Developer ID (conta paga da Apple); enquanto não há, a primeira abertura pede o `xattr` documentado no README.
