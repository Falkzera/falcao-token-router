//! O perfil de um grupo **compartilha** histórico e ferramentas com o
//! `~\.claude` (≙ `ProfileSharing.swift`, com os links do Windows).
//!
//! É o que torna `claude <grupo> --resume` capaz de listar as conversas que você
//! já tem, e traz skills, comandos e agentes para dentro do grupo. O
//! `settings.json` NÃO é compartilhado: o grupo precisa do seu (com a status line
//! do sensor), e compartilhá-lo faria a status line vazar para o `~\.claude`.
//!
//! No Windows:
//! - **pastas** (`skills`, `commands`, `agents`, e `projects` com histórico único)
//!   por **junction** — não pede privilégio;
//! - **arquivos** (`CLAUDE.md`, `keybindings.json`, e `history.jsonl` com
//!   histórico único) por **symlink**, que sem admin só sai com o Developer Mode.
//!   Sem ele, plano B: `CLAUDE.md`/`keybindings.json` viram cópias que o router
//!   mantém em dia (o mais novo vence, o lado sobrescrito vai para backup), e o
//!   `history.jsonl` (o ↑ de prompts) fica por grupo. Hardlink não: o binário
//!   2.1.280 reescreve o `history.jsonl` na poda de retenção quando é arquivo
//!   comum, e o hardlink divergiria em silêncio.
//!
//! Nunca sobrescreve o que já existe no grupo (a não ser a cópia que o próprio
//! router fez, registrada num manifesto), e ignora o que o `~\.claude` não tem.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use chrono::Utc;

use super::config_dir::ConfigDir;
use crate::platform::atomic_write::{read_retrying, write_atomic};
use crate::platform::links;

/// Tentar symlink de arquivo, ou ir direto para o plano B (testes).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SymlinkMode {
    Try,
    Never,
}

/// O que aconteceu, para o `doctor` e a UI explicarem.
#[derive(Clone, Debug, Default)]
pub struct SharingReport {
    /// O Windows deixou criar symlink de arquivo (Developer Mode ou admin).
    pub symlinks_available: bool,
    pub linked: Vec<String>,
    pub synced: Vec<String>,
}

/// `ERROR_PRIVILEGE_NOT_HELD`: symlink sem Developer Mode e sem admin.
const ERROR_PRIVILEGE_NOT_HELD: i32 = 1314;

/// Os arquivos que o router copiou para o grupo (plano B) — só esses entram na
/// sincronização; um arquivo que o usuário pôs lá de propósito, nunca.
const SYNC_MANIFEST: &str = ".falcao-router-sync.json";

static BACKUP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct ProfileSharing;

impl ProfileSharing {
    /// Seguem o usuário para o grupo sempre.
    pub const SHARED_DIRS: &'static [&'static str] = &["skills", "commands", "agents"];
    pub const SHARED_FILES: &'static [&'static str] = &["CLAUDE.md", "keybindings.json"];
    /// Histórico — só quando o usuário quer um histórico único (o padrão).
    pub const HISTORY_DIRS: &'static [&'static str] = &["projects"];
    pub const HISTORY_FILES: &'static [&'static str] = &["history.jsonl"];

    /// Liga o perfil do grupo ao `~\.claude` (`home`). Não faz nada no grupo
    /// padrão (o perfil dele JÁ É o `~\.claude`).
    pub fn link(
        group: &ConfigDir,
        home: &ConfigDir,
        share_history: bool,
        base: &Path,
    ) -> SharingReport {
        Self::link_with(group, home, share_history, SymlinkMode::Try, base)
    }

    pub fn link_with(
        group: &ConfigDir,
        home: &ConfigDir,
        share_history: bool,
        mode: SymlinkMode,
        base: &Path,
    ) -> SharingReport {
        let mut report = SharingReport::default();
        if group.is_default {
            return report;
        }
        let (group_dir, home_dir) = (group.path(), home.path());
        let _ = fs::create_dir_all(&group_dir);

        let history_dirs = if share_history {
            Self::HISTORY_DIRS
        } else {
            &[]
        };
        for name in Self::SHARED_DIRS.iter().chain(history_dirs) {
            let target = home_dir.join(name);
            let link = group_dir.join(name);
            if !target.is_dir() || occupied(&link) {
                continue;
            }
            if links::junction(&target, &link).is_ok() {
                report.linked.push((*name).to_string());
            }
        }

        let mut symlinks = mode == SymlinkMode::Try;
        let mut manifest = load_manifest(&group_dir);
        let manifest_before = manifest.clone();
        let history_files = if share_history {
            Self::HISTORY_FILES
        } else {
            &[]
        };
        for name in Self::SHARED_FILES.iter().chain(history_files) {
            let target = home_dir.join(name);
            let link = group_dir.join(name);
            if !target.is_file() {
                continue;
            }
            if symlinks && !occupied(&link) {
                match links::symlink_file(&target, &link) {
                    Ok(()) => {
                        report.linked.push((*name).to_string());
                        continue;
                    }
                    // Sem Developer Mode: plano B daqui em diante.
                    Err(e) if e.raw_os_error() == Some(ERROR_PRIVILEGE_NOT_HELD) => {
                        symlinks = false
                    }
                    Err(_) => continue,
                }
            }
            if symlinks || Self::HISTORY_FILES.contains(name) {
                continue; // já ligado, ou histórico que fica por grupo sem symlink
            }
            if sync_copy(name, &target, &link, &mut manifest, base) {
                report.synced.push((*name).to_string());
            }
        }
        if manifest != manifest_before {
            save_manifest(&group_dir, &manifest);
        }
        report.symlinks_available = symlinks;
        report
    }
}

/// Já há algo ali (pasta, arquivo, link — mesmo quebrado)?
fn occupied(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn load_manifest(group_dir: &Path) -> BTreeSet<String> {
    read_retrying(&group_dir.join(SYNC_MANIFEST))
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default()
}

fn save_manifest(group_dir: &Path, manifest: &BTreeSet<String>) {
    if let Ok(bytes) = serde_json::to_vec(manifest) {
        let _ = write_atomic(&group_dir.join(SYNC_MANIFEST), &bytes);
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Copia os bytes e **mantém o mtime da origem**: com mtime novo, o destino
/// pareceria "mais novo" e as duas cópias passariam a se sobrescrever em
/// pingue-pongue a cada lançamento.
fn copy_keeping_mtime(from: &Path, to: &Path) -> io::Result<()> {
    let bytes = read_retrying(from)?;
    write_atomic(to, &bytes)?;
    if let Some(when) = mtime(from) {
        OpenOptions::new()
            .write(true)
            .open(to)?
            .set_modified(when)?;
    }
    Ok(())
}

/// Guarda o arquivo que vai ser sobrescrito em `<base>\backups\sync-<ts>-<n>\`.
fn backup(path: &Path, side: &str, base: &Path) -> io::Result<()> {
    let n = BACKUP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = base.join("backups").join(format!(
        "sync-{}-{}-{n}",
        Utc::now().format("%Y%m%dT%H%M%S%.3fZ"),
        std::process::id()
    ));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    write_atomic(&dir.join(format!("{side}-{name}")), &read_retrying(path)?)
}

/// O plano B de um arquivo: cópia nova se o grupo não tem; se tem e foi o router
/// que copiou, o mais novo vence (com backup do outro); se é do usuário, nada.
fn sync_copy(
    name: &str,
    home_file: &Path,
    group_file: &Path,
    manifest: &mut BTreeSet<String>,
    base: &Path,
) -> bool {
    match fs::symlink_metadata(group_file) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            if copy_keeping_mtime(home_file, group_file).is_ok() {
                manifest.insert(name.to_string());
                return true;
            }
            false
        }
        Ok(meta) if meta.is_file() && manifest.contains(name) => {
            let (home_time, group_time) = (mtime(home_file), mtime(group_file));
            if group_time > home_time {
                backup(home_file, "perfil-padrao", base).is_ok()
                    && copy_keeping_mtime(group_file, home_file).is_ok()
            } else if home_time > group_time {
                backup(group_file, "grupo", base).is_ok()
                    && copy_keeping_mtime(home_file, group_file).is_ok()
            } else {
                false
            }
        }
        _ => false,
    }
}
