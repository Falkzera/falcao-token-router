//! Ajudantes dos testes de integração da CLI: um sandbox com base do router
//! (`ROUTER_APP_SUPPORT`), home (`USERPROFILE`), `APPDATA`, `PATH` mínimo e o
//! `fake-claude` por `ROUTER_CLAUDE_BIN`, e um "mundo" com um grupo dedicado e
//! contas logadas. Nenhuma conta real; nada fora do sandbox.
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use chrono::Utc;
use serde_json::Value;

use router_core::engine::group_usage::{GroupUsageSample, GroupUsageStore, UsageOrigin};
use router_core::engine::router_paths::RouterPaths;
use router_core::{Account, AccountGroup, AccountIdentity, ConfigDir, Id, Provider, RouterConfig};

/// Um blob com a forma do real (valores falsos).
pub fn blob(tag: &str) -> String {
    format!(r#"{{"claudeAiOauth":{{"accessToken":"falso-{tag}"}},"mcpOAuth":{{}}}}"#)
}

pub fn fake_claude() -> PathBuf {
    let path = assert_cmd::cargo::cargo_bin("fake-claude");
    assert!(
        path.is_file(),
        "fake-claude não foi compilado — rode `cargo test --workspace` (ou scripts\\test.ps1)"
    );
    path
}

pub struct Sandbox {
    _tmp: tempfile::TempDir,
    pub app: PathBuf,
    pub home: PathBuf,
    pub appdata: PathBuf,
    pub cwd: PathBuf,
    pub record: PathBuf,
}

impl Sandbox {
    pub fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let sandbox = Sandbox {
            app: root.join("app"),
            home: root.join("home"),
            appdata: root.join("appdata"),
            cwd: root.join("cwd"),
            record: root.join("fake-claude.jsonl"),
            _tmp: tmp,
        };
        for dir in [&sandbox.app, &sandbox.home, &sandbox.appdata, &sandbox.cwd] {
            fs::create_dir_all(dir).unwrap();
        }
        sandbox
    }

    pub fn paths(&self) -> RouterPaths {
        RouterPaths::with_app_support(Some(self.app.clone()))
    }

    /// O `router` com o ambiente do sandbox. `CLAUDE_CONFIG_DIR` sai: o teste
    /// pode estar rodando dentro de uma sessão do Claude Code, com o perfil real.
    pub fn router(&self, args: &[&str]) -> Command {
        let mut command = self.command(&assert_cmd::cargo::cargo_bin("router"));
        command.args(args);
        command
    }

    /// Qualquer programa com o ambiente do sandbox (o `PATH` só tem o System32).
    pub fn command(&self, program: &std::path::Path) -> Command {
        let system = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let mut command = Command::new(program);
        command
            .current_dir(&self.cwd)
            .env("ROUTER_APP_SUPPORT", &self.app)
            .env("USERPROFILE", &self.home)
            .env("HOME", &self.home)
            .env("APPDATA", &self.appdata)
            .env("PATH", format!(r"{system}\System32;{system}"))
            .env("ROUTER_CLAUDE_BIN", fake_claude())
            .env("FAKE_CLAUDE_RECORD", &self.record)
            .env_remove("CLAUDE_CONFIG_DIR")
            .env_remove("CLAUDE_SECURESTORAGE_CONFIG_DIR");
        command
    }

    pub fn run(&self, args: &[&str]) -> Output {
        self.router(args).output().unwrap()
    }

    /// As execuções do `fake-claude`, na ordem.
    pub fn records(&self) -> Vec<Value> {
        fs::read_to_string(&self.record)
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn identity(email: &str) -> AccountIdentity {
    let mut raw = serde_json::Map::new();
    raw.insert("emailAddress".into(), Value::String(email.into()));
    AccountIdentity {
        email: email.into(),
        organization_name: Some("Acme".into()),
        rate_limit_tier: None,
        raw,
    }
}

/// Um grupo dedicado "Trabalho" com `n` contas logadas (`conta1@…`, `conta2@…`),
/// cada uma com credencial e identidade na casa. Grupo padrão nunca.
pub struct World {
    pub sandbox: Sandbox,
    pub group: AccountGroup,
    pub accounts: Vec<Account>,
}

pub fn world(n: usize) -> World {
    let sandbox = Sandbox::new();
    let paths = sandbox.paths();
    let accounts: Vec<Account> = (1..=n)
        .map(|i| {
            let id = Id::new();
            let home = paths.account_home(id);
            fs::create_dir_all(home.path()).unwrap();
            fs::write(home.path().join(".credentials.json"), blob(&i.to_string())).unwrap();
            let email = format!("conta{i}@exemplo.com");
            fs::write(
                home.global_config_path(),
                format!(r#"{{"oauthAccount":{{"emailAddress":"{email}"}}}}"#),
            )
            .unwrap();
            Account {
                id,
                provider: Provider::Anthropic,
                identity: identity(&email),
                home,
                nickname: None,
            }
        })
        .collect();
    let mut group = AccountGroup::new("Trabalho", ConfigDir::dedicated(String::new()));
    group.config_dir = paths.group_config_dir(group.id, false, &sandbox.home.to_string_lossy());
    group.account_ids = accounts.iter().map(|a| a.id).collect();
    let config = RouterConfig {
        accounts: accounts.clone(),
        groups: vec![group.clone()],
        ..Default::default()
    };
    fs::create_dir_all(&paths.base).unwrap();
    fs::write(paths.config_file(), serde_json::to_vec(&config).unwrap()).unwrap();
    World {
        sandbox,
        group,
        accounts,
    }
}

impl World {
    pub fn group_dir(&self) -> PathBuf {
        self.group.config_dir.path()
    }

    pub fn group_identity(&self) -> Option<String> {
        let data = fs::read(self.group.config_dir.global_config_path()).ok()?;
        let root: Value = serde_json::from_slice(&data).ok()?;
        root["oauthAccount"]["emailAddress"]
            .as_str()
            .map(String::from)
    }

    pub fn write_sample(&self, email: &str, five: f64) {
        let sample = GroupUsageSample::new(
            self.group.config_dir.raw.clone(),
            Some(email.into()),
            Some(five),
            None,
            None,
            None,
            Utc::now(),
            None,
            UsageOrigin::Sensor,
        );
        GroupUsageStore::write(&sample, email, &self.sandbox.paths().usage_dir()).unwrap();
    }
}
