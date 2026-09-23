//! `ProfileSharing`: o perfil de um grupo compartilha histórico e ferramentas
//! com o `~\.claude`, para `claude <grupo> --resume` listar as conversas.
//!
//! Portados de `LauncherTests.swift` (suíte "ProfileSharing", 3). No Windows:
//! **pastas** por junction (não pede privilégio; `projects` é o que o `--resume`
//! lê); **arquivos** por symlink só quando o Windows deixa (Developer Mode). Sem
//! symlink, `CLAUDE.md`/`keybindings.json` viram cópias sincronizadas (mais novo
//! vence, com backup do sobrescrito) e o `history.jsonl` fica por grupo —
//! hardlink NÃO: o binário 2.1.280 reescreve o `history.jsonl` na poda de
//! retenção quando ele é arquivo comum, e o hardlink divergiria em silêncio.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use router_core::engine::profile_sharing::{ProfileSharing, SymlinkMode};
use router_core::platform::links::{is_junction, is_symlink};
use router_core::ConfigDir;

struct Homes {
    _tmp: tempfile::TempDir,
    home: ConfigDir,
    group: ConfigDir,
    base: std::path::PathBuf,
}

/// Um `~\.claude` de mentira com `projects`, `skills`, `history.jsonl` e
/// `CLAUDE.md`, e um perfil de grupo vazio.
fn homes() -> Homes {
    let tmp = tempfile::tempdir().unwrap();
    let home_dir = tmp.path().join(".claude");
    fs::create_dir_all(home_dir.join("projects")).unwrap();
    fs::create_dir_all(home_dir.join("skills")).unwrap();
    fs::write(home_dir.join("history.jsonl"), "x\n").unwrap();
    fs::write(home_dir.join("CLAUDE.md"), "casa").unwrap();
    let base = tmp.path().join("base");
    Homes {
        home: ConfigDir::dedicated(home_dir.to_string_lossy().into_owned()),
        group: ConfigDir::dedicated(tmp.path().join("grupo").to_string_lossy().into_owned()),
        base,
        _tmp: tmp,
    }
}

fn set_mtime(path: &Path, when: SystemTime) {
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(when)
        .unwrap();
}

#[test]
fn projects_and_history_are_shared_when_history_is_shared() {
    let h = homes();
    let report = ProfileSharing::link_with(&h.group, &h.home, true, SymlinkMode::Try, &h.base);

    let g = h.group.path();
    assert!(
        is_junction(&g.join("projects")),
        "projects devia ser junction"
    );
    assert!(is_junction(&g.join("skills")), "skills devia ser junction");
    // Arquivos: symlink quando o Windows deixa; senão cópia (CLAUDE.md) ou nada
    // (history.jsonl fica por grupo).
    if report.symlinks_available {
        assert!(is_symlink(&g.join("history.jsonl")));
        assert!(is_symlink(&g.join("CLAUDE.md")));
    } else {
        assert!(!g.join("history.jsonl").exists());
        assert_eq!(fs::read_to_string(g.join("CLAUDE.md")).unwrap(), "casa");
    }
}

#[test]
fn without_shared_history_projects_is_not_linked() {
    let h = homes();
    ProfileSharing::link_with(&h.group, &h.home, false, SymlinkMode::Try, &h.base);

    let g = h.group.path();
    assert!(!g.join("projects").exists());
    assert!(!g.join("history.jsonl").exists());
    // Mas skills (não é histórico) segue vindo.
    assert!(is_junction(&g.join("skills")));
}

/// O perfil do grupo padrão JÁ É o `~\.claude`: ligar para si mesmo seria
/// circular.
#[test]
fn the_default_group_is_left_alone() {
    let h = homes();
    let default = ConfigDir::standard(&h.group.path().to_string_lossy());
    ProfileSharing::link_with(&default, &h.home, true, SymlinkMode::Try, &h.base);
    assert!(!default.path().exists());
}

// --- Regressões do Windows ---

/// O que já existe no grupo (pasta ou arquivo reais) nunca é trocado por link.
#[test]
fn a_real_item_in_the_group_is_never_replaced() {
    let h = homes();
    let g = h.group.path();
    fs::create_dir_all(g.join("projects")).unwrap();
    fs::write(g.join("projects").join("meu.jsonl"), "do grupo").unwrap();

    ProfileSharing::link_with(&h.group, &h.home, true, SymlinkMode::Try, &h.base);

    assert!(!is_junction(&g.join("projects")));
    assert_eq!(
        fs::read_to_string(g.join("projects").join("meu.jsonl")).unwrap(),
        "do grupo"
    );
}

/// É a junction que faz o `--resume` do grupo enxergar tudo: um transcript
/// gravado pelo grupo cai no `~\.claude\projects`.
#[test]
fn a_transcript_written_through_the_junction_lands_in_the_home() {
    let h = homes();
    ProfileSharing::link_with(&h.group, &h.home, true, SymlinkMode::Try, &h.base);

    let project = h.group.path().join("projects").join("C--Users-exemplo-app");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("sessao.jsonl"), "{}\n").unwrap();

    assert!(h
        .home
        .path()
        .join("projects")
        .join("C--Users-exemplo-app")
        .join("sessao.jsonl")
        .exists());
}

/// Sem symlink: o `CLAUDE.md` do grupo é uma cópia que o router mantém em dia —
/// o mais novo vence nos dois sentidos, e o lado sobrescrito vai para backup.
#[test]
fn without_symlinks_claude_md_is_synced_newest_wins_with_a_backup() {
    let h = homes();
    let g = h.group.path();
    ProfileSharing::link_with(&h.group, &h.home, true, SymlinkMode::Never, &h.base);
    assert_eq!(fs::read_to_string(g.join("CLAUDE.md")).unwrap(), "casa");
    assert!(
        !g.join("history.jsonl").exists(),
        "history.jsonl fica por grupo"
    );

    // O usuário edita no grupo (mais novo): vai para a casa, com backup.
    let later = SystemTime::now() + Duration::from_secs(60);
    fs::write(g.join("CLAUDE.md"), "editado no grupo").unwrap();
    set_mtime(&g.join("CLAUDE.md"), later);
    ProfileSharing::link_with(&h.group, &h.home, true, SymlinkMode::Never, &h.base);
    assert_eq!(
        fs::read_to_string(h.home.path().join("CLAUDE.md")).unwrap(),
        "editado no grupo"
    );

    // Depois edita na casa (mais nova ainda): vem para o grupo, com backup.
    fs::write(h.home.path().join("CLAUDE.md"), "editado na casa").unwrap();
    set_mtime(
        &h.home.path().join("CLAUDE.md"),
        later + Duration::from_secs(60),
    );
    ProfileSharing::link_with(&h.group, &h.home, true, SymlinkMode::Never, &h.base);
    assert_eq!(
        fs::read_to_string(g.join("CLAUDE.md")).unwrap(),
        "editado na casa"
    );

    let backups: Vec<String> = walk(&h.base.join("backups"))
        .into_iter()
        .map(|p| fs::read_to_string(p).unwrap())
        .collect();
    assert!(backups.contains(&"casa".to_string()), "{backups:?}");
    assert!(
        backups.contains(&"editado no grupo".to_string()),
        "{backups:?}"
    );
}

/// Um `CLAUDE.md` que o usuário pôs no grupo de propósito (não foi o router que
/// copiou) nunca entra na sincronização.
#[test]
fn a_users_own_file_in_the_group_is_never_synced() {
    let h = homes();
    let g = h.group.path();
    fs::create_dir_all(&g).unwrap();
    fs::write(g.join("CLAUDE.md"), "só deste grupo").unwrap();
    set_mtime(
        &g.join("CLAUDE.md"),
        SystemTime::now() + Duration::from_secs(60),
    );

    ProfileSharing::link_with(&h.group, &h.home, true, SymlinkMode::Never, &h.base);

    assert_eq!(
        fs::read_to_string(g.join("CLAUDE.md")).unwrap(),
        "só deste grupo"
    );
    assert_eq!(
        fs::read_to_string(h.home.path().join("CLAUDE.md")).unwrap(),
        "casa"
    );
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}
