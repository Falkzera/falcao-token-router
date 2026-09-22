# Presentation — agent.md

## Propósito
A geometria que a UI desenha, **sem SwiftUI**. Mora no core pela mesma razão que o resto: assim a regra é testável sem instanciar janela.

## Arquivos
- `GaugeGeometry.swift` — o caminho do anel-medidor. A `Shape` no alvo de UI só desenha o que estas funções decidem.

## Padrões
- Nada aqui importa SwiftUI. Se precisar, é sinal de que pertence ao alvo do app.

## Decisões recentes
- A marca existe UMA vez: a barra de menus desenha com a fração ao vivo, e `Scripts/icon.swift` — compilado contra este mesmo arquivo — congela em 62% para o ícone do app.

## Pendências conhecidas
- Nenhuma.
