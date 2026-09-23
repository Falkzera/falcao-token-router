# app/src-tauri/src — o backend do app

## Arquivos
- `main.rs` — só chama `run()`. Sem console em release (o app vive na bandeja).
- `lib.rs` — o `Builder`: instância única (1º plugin; a 2ª execução abre a janela), a janela
  `home` (520×620 fixa) e os comandos. Hoje: `app_info` (versão + idioma).
- `locale.rs` — o idioma da interface do Windows (`GetUserDefaultUILanguage`): qualquer
  português → catálogo pt-BR; o resto → en. O front recebe daqui, para bandeja e janelas
  nunca discordarem.

## Pendências (próximas fatias)
- 5.2 bandeja (anel + tooltip + menu), laço de 180 s, cura da integração na subida.
- 5.3 flyout; 5.4 comandos da janela; 5.5 login por ConPTY.
