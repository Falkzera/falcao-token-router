# Presentation — agent.md

## Propósito
A geometria que a UI desenha, **sem SwiftUI**. Mora no core pela mesma razão que o resto: assim a regra é testável sem instanciar janela.

## Arquivos
- `GaugeGeometry.swift` — o caminho do anel-medidor. A `Shape` no alvo de UI só desenha o que estas funções decidem.

## Padrões
- Nada aqui importa SwiftUI. Se precisar, é sinal de que pertence ao alvo do app.

## Decisões recentes
- 2026-10-05: o anel é o medidor, não a marca. A barra de menus o desenha com a fração ao vivo; o ícone do app, o banner e o card social são da marca Falcão (`Scripts/icon.swift`), que não compila mais contra este arquivo.

## Pendências conhecidas
- Nenhuma.
