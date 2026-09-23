//! A escolha do usuário sobre a status line dos grupos. A aba Ajustes do app a
//! grava; o `router statusline` a lê a CADA render.
//!
//! - modo `app` (o de fábrica): a linha completa, menos os itens tirados;
//! - modo `command`: o router mede como sempre e depois roda o comando do
//!   usuário com o mesmo JSON (sem saída, vale a linha do app).
//!
//! Mora em `<base>\statusline.json`, a pasta que o app e a CLI já dividem: os
//! ajustes do app ficam na Roaming, que a CLI não lê, e o `config.json` é o
//! formato combinado com o macOS. A leitura é TOLERANTE — arquivo ausente,
//! ilegível ou de outro formato vale a de fábrica; item desconhecido é
//! ignorado —: a status line nunca falha por causa da escolha.

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::platform::atomic_write::{read_retrying, write_atomic};

/// Um item da linha completa, na ordem em que ela os desenha.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Item {
    Group,
    Model,
    Effort,
    Place,
    Context,
    FiveHour,
    SevenDay,
    Resets,
    Cost,
    Email,
}

impl Item {
    /// Todos, na ordem da linha.
    pub const ALL: [Item; 10] = [
        Item::Group,
        Item::Model,
        Item::Effort,
        Item::Place,
        Item::Context,
        Item::FiveHour,
        Item::SevenDay,
        Item::Resets,
        Item::Cost,
        Item::Email,
    ];

    /// O nome no arquivo (e na ponte com a tela).
    pub fn key(self) -> &'static str {
        match self {
            Item::Group => "group",
            Item::Model => "model",
            Item::Effort => "effort",
            Item::Place => "place",
            Item::Context => "context",
            Item::FiveHour => "fiveHour",
            Item::SevenDay => "sevenDay",
            Item::Resets => "resets",
            Item::Cost => "cost",
            Item::Email => "email",
        }
    }

    pub fn from_key(key: &str) -> Option<Item> {
        Item::ALL.into_iter().find(|item| item.key() == key)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    /// A linha do app, com os itens escolhidos.
    #[default]
    App,
    /// O comando do usuário, depois do sensor.
    Command,
}

/// O que está em `statusline.json`. Passa pelo `serde` como o arquivo é (o
/// app manda à tela e recebe dela o mesmo formato), sempre pelo caminho
/// tolerante: um JSON válido nunca é recusado.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(from = "Value", into = "Value")]
pub struct StatusLineChoice {
    pub mode: Mode,
    /// Os itens TIRADOS da linha completa, na ordem dela e sem repetição —
    /// não os que aparecem: um item novo numa versão futura aparece para
    /// todos, como "a completa menos o que eu tirei".
    hidden: Vec<Item>,
    /// Guardado também no modo `app`: voltar ao modo comando o traz de volta.
    pub command: String,
}

impl From<Value> for StatusLineChoice {
    fn from(value: Value) -> Self {
        let mode = match value.get("mode").and_then(Value::as_str) {
            Some("command") => Mode::Command,
            _ => Mode::App,
        };
        let mut choice = StatusLineChoice {
            mode,
            hidden: Vec::new(),
            command: value
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        };
        for item in value
            .get("hidden")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().and_then(Item::from_key))
        {
            choice.set_shown(item, false);
        }
        choice
    }
}

impl From<StatusLineChoice> for Value {
    fn from(choice: StatusLineChoice) -> Self {
        let mode = match choice.mode {
            Mode::App => "app",
            Mode::Command => "command",
        };
        let hidden: Vec<&str> = choice.hidden.iter().map(|item| item.key()).collect();
        json!({"mode": mode, "hidden": hidden, "command": choice.command})
    }
}

impl StatusLineChoice {
    /// A escolha gravada, ou a de fábrica (a completa) se não houver uma que
    /// se leia.
    pub fn load(path: &Path) -> Self {
        read_retrying(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// Grava atômica (temporário + renomear): a CLI, que lê a cada render,
    /// nunca pega o arquivo pela metade.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        write_atomic(path, &bytes)
    }

    pub fn shows(&self, item: Item) -> bool {
        !self.hidden.contains(&item)
    }

    pub fn set_shown(&mut self, item: Item, shown: bool) {
        self.hidden.retain(|&hidden| hidden != item);
        if !shown {
            self.hidden.push(item);
            self.hidden
                .sort_by_key(|hidden| Item::ALL.iter().position(|i| i == hidden));
        }
    }

    pub fn hidden(&self) -> &[Item] {
        &self.hidden
    }

    /// O comando a rodar depois do sensor: só no modo comando, e só se não
    /// estiver em branco (em branco, vale a linha do app).
    pub fn command_to_run(&self) -> Option<&str> {
        match self.mode {
            Mode::Command => Some(self.command.trim()).filter(|c| !c.is_empty()),
            Mode::App => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::router_paths::RouterPaths;

    fn file_with(text: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("statusline.json");
        std::fs::write(&path, text).unwrap();
        (tmp, path)
    }

    #[test]
    fn the_factory_choice_is_the_full_app_line() {
        let choice = StatusLineChoice::default();
        assert_eq!(choice.mode, Mode::App);
        assert!(Item::ALL.iter().all(|&item| choice.shows(item)));
        assert_eq!(choice.command_to_run(), None);
    }

    #[test]
    fn a_missing_file_is_the_full_line() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            StatusLineChoice::load(&tmp.path().join("statusline.json")),
            StatusLineChoice::default()
        );
    }

    /// Meio escrito, editado à mão, de outra versão: a status line nunca falha
    /// por causa da escolha — vale a completa.
    #[test]
    fn an_unreadable_or_strange_file_is_the_full_line() {
        for text in [
            "{ nao e json",
            "",
            "[1, 2]",
            "\"app\"",
            r#"{"mode": 3, "hidden": "context", "command": false}"#,
        ] {
            let (_tmp, path) = file_with(text);
            assert_eq!(
                StatusLineChoice::load(&path),
                StatusLineChoice::default(),
                "{text}"
            );
        }
    }

    /// Um item de uma versão mais nova some sem levar os outros junto.
    #[test]
    fn unknown_items_are_ignored_and_known_ones_kept() {
        let (_tmp, path) = file_with(r#"{"hidden": ["context", "linesChanged", 7, "cost"]}"#);
        let choice = StatusLineChoice::load(&path);
        assert_eq!(choice.hidden(), [Item::Context, Item::Cost]);
        assert_eq!(choice.mode, Mode::App);
    }

    #[test]
    fn an_unknown_mode_is_the_app_line_and_keeps_the_items() {
        let (_tmp, path) = file_with(r#"{"mode": "fancy", "hidden": ["email"]}"#);
        let choice = StatusLineChoice::load(&path);
        assert_eq!(choice.mode, Mode::App);
        assert_eq!(choice.hidden(), [Item::Email]);
    }

    #[test]
    fn hidden_items_keep_the_line_order_without_repeats() {
        let (_tmp, path) = file_with(r#"{"hidden": ["email", "context", "email"]}"#);
        assert_eq!(
            StatusLineChoice::load(&path).hidden(),
            [Item::Context, Item::Email]
        );
    }

    #[test]
    fn showing_and_hiding_an_item() {
        let mut choice = StatusLineChoice::default();
        choice.set_shown(Item::Context, false);
        choice.set_shown(Item::Group, false);
        choice.set_shown(Item::Context, false);
        assert!(!choice.shows(Item::Context) && !choice.shows(Item::Group));
        assert_eq!(choice.hidden(), [Item::Group, Item::Context]);
        choice.set_shown(Item::Group, true);
        assert_eq!(choice.hidden(), [Item::Context]);
    }

    /// O que o app grava é o que a CLI lê — e a gravação não deixa temporário.
    #[test]
    fn the_choice_survives_a_save_and_a_load() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("base").join("statusline.json");
        let mut choice = StatusLineChoice {
            mode: Mode::Command,
            command: "node C:/Users/exemplo/linha.js".into(),
            ..Default::default()
        };
        choice.set_shown(Item::Cost, false);
        choice.set_shown(Item::Context, false);

        choice.save(&path).unwrap();
        assert_eq!(StatusLineChoice::load(&path), choice);
        let names: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["statusline.json"]);
    }

    /// O arquivo é legível por gente: os nomes dos itens, na ordem da linha.
    #[test]
    fn the_file_speaks_item_names() {
        let mut choice = StatusLineChoice::default();
        choice.set_shown(Item::Email, false);
        choice.set_shown(Item::FiveHour, false);
        assert_eq!(
            serde_json::to_value(&choice).unwrap(),
            serde_json::json!({"mode": "app", "hidden": ["fiveHour", "email"], "command": ""})
        );
        let command = StatusLineChoice {
            mode: Mode::Command,
            ..Default::default()
        };
        assert_eq!(serde_json::to_value(&command).unwrap()["mode"], "command");
    }

    #[test]
    fn every_item_has_its_own_name() {
        for item in Item::ALL {
            assert_eq!(Item::from_key(item.key()), Some(item));
        }
        let mut keys: Vec<_> = Item::ALL.iter().map(|i| i.key()).collect();
        keys.dedup();
        assert_eq!(keys.len(), Item::ALL.len());
        assert_eq!(Item::from_key("Context"), None);
    }

    /// O comando só roda no modo comando e se não estiver em branco — senão,
    /// a linha do app.
    #[test]
    fn the_command_runs_only_in_command_mode_and_when_not_blank() {
        let with = |mode, command: &str| StatusLineChoice {
            mode,
            command: command.into(),
            ..Default::default()
        };
        assert_eq!(with(Mode::App, "node linha.js").command_to_run(), None);
        assert_eq!(with(Mode::Command, "   ").command_to_run(), None);
        assert_eq!(
            with(Mode::Command, " node linha.js \n").command_to_run(),
            Some("node linha.js")
        );
    }

    #[test]
    fn the_file_lives_in_the_router_base() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = RouterPaths::with_app_support(Some(tmp.path().to_path_buf()));
        assert_eq!(
            paths.status_line_file(),
            tmp.path()
                .join("com.synqo.falcao-router")
                .join("statusline.json")
        );
    }
}
