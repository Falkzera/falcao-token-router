# app/src-tauri/src — o backend do app

## Arquivos
- `main.rs` — só chama `run()`. Sem console em release (o app vive na bandeja).
- `lib.rs` — o `Builder`: instância única (1º plugin; a 2ª execução abre a janela, e a restaura
  se estiver minimizada), o estado, a cura da integração na subida (`attach_router`), a bandeja,
  o laço, a janela `home` (520×620 fixa; abre sozinha na subida sem grupo nenhum OU com "Mostrar
  na barra de tarefas" — `AppSettings::present_at_launch`, ≙ `presentAtLaunch` do macOS) e os
  comandos (`app_info`). Fechar a janela não encerra o app — só o "Sair" (saída com código); com
  "Mostrar na barra de tarefas", fechar só MINIMIZA (`CloseRequested` → `prevent_close`), e a
  preferência é lida na hora do fechar. A janela destruída encerra o login que estivesse nela
  (ninguém veria o desfecho).
- `state.rs` — `AppState`: o `RouterConfigStore` (montado como na CLI: credencial em arquivo com a
  guarda do perfil padrão) atrás de um `Mutex` que sobrevive a envenenamento, o idioma, a home e
  a aba pedida para a próxima abertura da janela.
- `tray.rs` — a bandeja: anel do `gauge-mark` no tamanho exato do ícone pequeno (cache por
  `TrayKey`), tooltip, menu do botão direito (Grupos, Ajustes, Sair — as portas do rodapé do
  painel do macOS); clique esquerdo (na soltura) abre o flyout junto do ícone.
- `flyout.rs` — a janela do flyout: criada escondida na subida (1º clique imediato), 330 px,
  sem borda, com sombra, sempre por cima, fora da barra de tarefas; some ao perder o foco; o
  clique que a fechou não a reabre (guarda de 300 ms); `position` (pura, testada): acima da
  barra embaixo, abaixo dela em cima, ao lado nas laterais, acima do clique quando o ícone está
  no excedente, sempre dentro da área útil do monitor; a altura segue o conteúdo. Abrir relê o
  quadro (como o painel do macOS).
- `commands.rs` — os comandos da janela de Grupos: cada ação muda o store (sob a trava), redesenha
  a bandeja, emite `snapshot-changed` e devolve o quadro novo. "Medir contas" roda a sonda numa
  thread FORA da trava (planeja → roda → publica), uma medição por vez, com o spinner no quadro.
  Grupo novo com a escolha de padrão (`add_group_with`); login estranho no `~\.claude`
  (`foreign_default_login`); dispensar o erro. `copy_text` (plugin clipboard) mora no `lib.rs`.
- `terminal.rs` — a integração de terminal na tela: o quadro POR SHELL (`terminal_report`, fora
  da thread da interface — consulta a política de cada PowerShell), "Ativar/Reinstalar"
  (`install_integration`, devolve `ok` de verdade: o "Instalada ✓" do macOS aparecia mesmo com
  falha) e a correção consentida da política (`allow_profiles_for`: RemoteSigned em CurrentUser,
  conferido de novo depois). `needs_install` diz se "Ativar" resolve algo (scripts ausentes/velhos
  ou shell sem a linha — a política e o `.bash_profile` que ignora o `.bashrc` têm correção
  própria); `bash_login_file` nomeia o perfil de login do Git Bash. Testado (`view`).
- `settings.rs` — ajustes do app em `<Roaming>\com.synqo.falcao-token-router\settings.json`
  (`show_in_taskbar`; ilegível = padrão); "abrir no login" pelo plugin de autostart, SEMPRE relido
  do sistema, com o motivo da recusa; `open_url` só para a lista (ms-settings:developers/taskbar,
  logout do claude.ai, hosts do login oficial — testado). `SettingsStore::at` para os testes.
- `snapshot.rs` — o quadro que as janelas leem: grupos e contas na ordem do usuário, conta
  ativa, uso com janela/origem/idade e os % PRONTOS (`UsagePercent` do núcleo), sessões (total
  e engajadas), contas exclusivas (para a confirmação de apagar grupo), o comando do terminal e
  o erro da última ação como fato com código (`ErrorView`).
- `tray_text.rs` — o que a bandeja diz, lógica pura: o anel segue a conta do grupo PADRÃO (sem
  ativa nele, o primeiro grupo com ativa); uma linha por grupo com **janela, origem e idade**
  (`Trabalho: equipe-2 · 7d 81% (sensor, 3m)`); janela por modelo diz "sonda" e a idade da
  sonda; conta sem amostra = "pronta" e anel vazio; linhas inteiras até o limite de 127
  unidades UTF-16 do Windows.
- `rotation_loop.rs` — a cada 30 s relê o quadro e redesenha a bandeja; a cada 180 s (6 voltas)
  também rotaciona, como o macOS (a 1ª volta, com rotação, é na subida). Emite
  `snapshot-changed` para as janelas. Com um login em andamento a rotação fica para a próxima
  volta: ela espelha grupo → casa da conta ativa e pisaria na credencial nova de um relogin.
- `login_output.rs` — o que a saída do `claude auth login` diz, lida através do ConPTY (puro,
  testado, com o fluxo REAL gravado): limpa VT/ANSI em fluxo (sequência cortada entre pedaços
  não vaza), pega o link pelo texto visível (só completo, só nos dois hosts oficiais, nunca
  dentro de outro link) ou pelo hyperlink OSC 8 (o ConPTY o re-emite como `ESC]8;id=…;URL ESC\`),
  e emite uma vez cada fato: link, código inválido, sucesso, "Press Enter", falha com motivo.
  Conta os `ESC[6n` (pedido de posição do cursor) para o driver responder.
- `login_session.rs` — o driver: `claude auth login` (+ `--email` no relogin) num ConPTY de
  2048 colunas (o link não quebra), com `env_clear` + o ambiente do login (`ProviderEnv::direct`
  + `without_nested_session` + a casa no `CLAUDE_CONFIG_DIR` + `TERM`) — sem o `env_clear` o
  `portable-pty` relê as variáveis do REGISTRO. Responde `ESC[1;1R` ao `ESC[6n` (o
  `portable-pty` usa `PSEUDOCONSOLE_INHERIT_CURSOR` e o ConPTY segura a saída até a resposta);
  o código colado vai como linha + `\r`; o fim do processo só é contado depois de toda a saída
  lida; `cancel_and_wait` espera o `claude` sair antes de alguém mexer na casa. Testado no
  ConPTY de verdade com o `fake-claude`.
- `login.rs` — o fluxo do login no app: fases (iniciando, link, conferindo, adicionada,
  renovada, duplicada, conta errada, falha com motivo, tempo esgotado), a conferência NO DISCO
  (12×400 ms, numa thread), comandos (`start_login`, `start_relogin`, `current_login`,
  `login_submit_code`, `login_retry`, `login_recheck`, `login_close`) e o evento
  `login-changed` com revisão crescente (a resposta de um comando pode chegar depois do evento
  de uma mudança posterior). A transição de fase, a limpeza ao fechar e o plano do "Tentar de
  novo" são funções puras testadas: conta nova que não virou conta → a casa reservada sai;
  relogin nunca apaga a casa, só tira o login estranho de um relogin que voltou com outra conta;
  "Tentar de novo" da conta nova é numa casa NOVA.
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

## Verificado à mão (23/09/2026, sandbox)
- "Mostrar na barra de tarefas" ligado: a janela abre na subida mesmo com grupos; `WM_CLOSE` (o X)
  a deixa minimizada e o processo vivo; a 2ª execução a restaura. Desligado: nada abre na subida
  (há grupos), a 2ª execução abre, `WM_CLOSE` a destrói e o app segue na bandeja.
- O `USERPROFILE` falso do sandbox ISOLA a Roaming/Local (o registro as guarda como
  `%USERPROFILE%\AppData\…`, expandido com o ambiente do processo): o `settings.json` do app de
  teste mora na home falsa. NÃO isola a Documentos redirecionada ao OneDrive (caminho absoluto —
  os `$PROFILE` são os reais) nem o HKCU (o autostart escreve no `Run` real).

- Login pelo app de verdade (sandbox; o `claude` era o `fake-claude` por `ROUTER_CLAUDE_BIN`; a
  janela dirigida pelo DevTools do WebView2 com `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=
  --remote-debugging-port`, sem mover o mouse): adicionar (a conta entra no grupo, casa com
  credencial e identidade), relogar ("Login renovado!"), cancelar (a casa reservada sai do disco
  e nenhum `claude` fica vivo) e relogin com outra conta (ao fechar, a credencial estranha sai
  da casa; a conta continua registrada).

## Pendências (próximas fatias)
- O login com o `claude` de verdade (navegador, conta real) é do teste ponta a ponta (fase 7).
- Conferir à mão o flyout no clique real da bandeja (posição com a barra embaixo e no excedente
  do Windows 11, sumir ao perder o foco, clique que fecha não reabre) — o clique no ícone não
  se automatiza sem mover o mouse do usuário.
