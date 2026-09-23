# app/src-tauri/src — o backend do app

## Arquivos
- `main.rs` — só chama `run()`. Sem console em release (o app vive na bandeja).
- `lib.rs` — o `Builder`: instância única (1º plugin; a 2ª execução abre a janela), o estado, a
  cura da integração na subida (`attach_router`), a bandeja, o laço, a janela `home` (520×620
  fixa; abre sozinha só na 1ª execução, sem grupo) e os comandos (`app_info`). Fechar a janela
  não encerra o app — só o "Sair" (saída com código).
- `state.rs` — `AppState`: o `RouterConfigStore` (montado como na CLI: credencial em arquivo com a
  guarda do perfil padrão) atrás de um `Mutex` que sobrevive a envenenamento, o idioma, a home e
  a aba pedida para a próxima abertura da janela.
- `tray.rs` — a bandeja: anel do `gauge-mark` no tamanho exato do ícone pequeno (cache por
  `TrayKey`), tooltip, menu do botão direito (Grupos, Ajustes, Sair — as portas do rodapé do
  painel do macOS); clique esquerdo abre a janela (o flyout vem na 5.3).
- `tray_text.rs` — o que a bandeja diz, lógica pura: o anel segue a conta do grupo PADRÃO (sem
  ativa nele, o primeiro grupo com ativa); uma linha por grupo com **janela, origem e idade**
  (`Trabalho: equipe-2 · 7d 81% (sensor, 3m)`); janela por modelo diz "sonda" e a idade da
  sonda; conta sem amostra = "pronta" e anel vazio; linhas inteiras até o limite de 127
  unidades UTF-16 do Windows.
- `rotation_loop.rs` — a cada 30 s relê o quadro e redesenha a bandeja; a cada 180 s (6 voltas)
  também rotaciona, como o macOS (a 1ª volta, com rotação, é na subida). Emite
  `snapshot-changed` para as janelas.
- `i18n.rs` — os MESMOS catálogos do front (`include_str!`), com o mesmo preenchimento de
  placeholder (`%@`, `%d`, `%1$@`, `%%`); `t(idioma, chave, args)`.
- `system.rs` — tema da BARRA de tarefas (`SystemUsesLightTheme`), tamanho do ícone pequeno na
  escala do sistema, o `router.exe` ao lado do app, o shell da status line.
- `locale.rs` — o idioma da interface do Windows: qualquer português → pt-BR; o resto → en.

## Verificado à mão (22/09/2026, sandbox com contas @exemplo.com)
- O Windows 11 registrou o ícone com o tooltip esperado (`HKCU\Control Panel\NotifyIconSettings`,
  `InitialTooltip`): `Trabalho: equipe-2 · 7d 81% (sensor, 3m)` / `Pessoal: conta1 · pronta`.
- 2ª execução sai com 0 e abre a janela na 1ª; fechar a janela deixa o app vivo.
- Ícone novo cai no excedente (`^`) da bandeja — daí a dica de fixar e a opção da barra de tarefas.

## Pendências (próximas fatias)
- 5.3 flyout no clique esquerdo; 5.4 comandos da janela; 5.5 login por ConPTY.
