# gauge-mark — a marca: o anel-medidor (≙ GaugeGeometry + Scripts/icon.swift)

Um desenho só para a bandeja (fração ao vivo) e para o ícone do app (congelado em 62%). Sem
Tauri e sem sistema: devolve pixels RGBA (sem pré-multiplicação) e PNG — testável por pixel.

## Arquivos
- `lib.rs` — `quantize` (20 passos; pontas protegidas: cheio só em 100%, vazio só em 0),
  `severity` (limiares do painel: 0,66 aviso, 0,90 crítico), `TrayKey`/`render_tray`/`tray_icon`
  (a chave do cache da bandeja: passo, cor, tema, tamanho), `tray_png` (prévia), `app_icon`/
  `app_icon_png`/`app_icon_ico` (superelipse verde + anel branco; abaixo de 64 px o traço
  engrossa e brilho/fio de luz saem).
- `bin/icongen.rs` — gera os ícones que o `tauri.conf.json` lista (`icon.ico` com cada tamanho
  desenhado no próprio tamanho, os PNGs) e, com `--tray`, a prévia da bandeja nos estados e temas.

## Decisões
- 22/09/2026: no estado calmo o anel da bandeja tem a cor do TEMA DA BARRA (branco na escura,
  quase preto na clara), não verde — paridade com `UsageColor.menuBar` do macOS: ícone sempre
  visível não compete com o resto da bandeja; a cor só entra no aviso e no crítico. A trilha é
  a mesma cor a 30%. Sem amostra ("pronta") = só a trilha.
- 22/09/2026: a cor vem da fração REAL e o desenho da QUANTIZADA (66% já é aviso, como no
  painel, mesmo caindo no passo de 65%).
- 22/09/2026: o `.ico` é montado aqui (entradas PNG, que o Windows lê desde o Vista) em vez de
  pelo `tauri icon`, que redimensiona o de 1024 px — a 16 px daria o anel borrado que o modo
  compacto existe para evitar.
- Arco das 12 horas no sentido HORÁRIO, em cúbicas de até 90° (o tiny-skia não tem arco).

## Regerar os ícones
`cargo run -p gauge-mark --bin icongen -- app\src-tauri\icons` (a partir de `windows\`).
