# gauge-mark — o anel-medidor (≙ GaugeGeometry)

O desenho da bandeja: a fração ao vivo da conta que o grupo usa. Sem Tauri e sem sistema:
devolve pixels RGBA (sem pré-multiplicação) e PNG — testável por pixel.

O ícone do app não sai mais daqui: desde 05/10/2026 é da marca Falcão (o símbolo branco sobre
marinho), em `app/src-tauri/icons/`. O anel mede, o falcão assina.

## Arquivos
- `lib.rs` — `quantize` (20 passos; pontas protegidas: cheio só em 100%, vazio só em 0),
  `severity` (limiares do painel: 0,66 aviso, 0,90 crítico), `TrayKey`/`render_tray`/`tray_icon`
  (a chave do cache da bandeja: passo, cor, tema, tamanho), `tray_png` (prévia).
- `bin/icongen.rs` — a prévia da bandeja nos estados e temas (`--tray`, o modo antigo, segue
  aceito).

## Decisões
- 22/09/2026: no estado calmo o anel da bandeja tem a cor do TEMA DA BARRA (branco na escura,
  quase preto na clara), não verde — paridade com `UsageColor.menuBar` do macOS: ícone sempre
  visível não compete com o resto da bandeja; a cor só entra no aviso e no crítico. A trilha é
  a mesma cor a 30%. Sem amostra ("pronta") = só a trilha.
- 22/09/2026: a cor vem da fração REAL e o desenho da QUANTIZADA (66% já é aviso, como no
  painel, mesmo caindo no passo de 65%).
- Arco das 12 horas no sentido HORÁRIO, em cúbicas de até 90° (o tiny-skia não tem arco).
- 05/10/2026: saíram `app_icon`, `app_icon_png`, `app_icon_ico` e `ICON_FRACTION`. O ícone do
  app passou a ser a marca Falcão, e um `icongen` que ainda o gerasse devolveria o anel por cima
  dos ícones da marca.

## Prévia da bandeja
`cargo run -p gauge-mark --bin icongen -- <pasta>` (a partir de `windows\`).
