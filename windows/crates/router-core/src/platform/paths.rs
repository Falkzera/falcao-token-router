//! Comparação de caminhos do jeito do Windows, sem tocar o disco.

use std::path::{Component, Path};

/// Forma de comparação: sem caixa, `/` = `\`, sem barra no fim. Não resolve
/// link nem `8.3` — é comparação de texto, para o que o próprio router gravou.
pub fn normalized(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

/// `child` fica DENTRO de `parent` (e não é o próprio)? Qualquer `..` no
/// `child` desqualifica: `<base>\accounts\..\..\x` não está dentro de nada.
pub fn is_strictly_inside(child: &Path, parent: &Path) -> bool {
    if child
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return false;
    }
    let (child, parent) = (normalized(child), normalized(parent));
    child.len() > parent.len() + 1
        && child.starts_with(&parent)
        && child.as_bytes()[parent.len()] == b'\\'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_ignores_case_and_slash_style() {
        let base = Path::new(r"C:\Users\exemplo\AppData\Local\com.synqo.falcao-router\accounts");
        assert!(is_strictly_inside(
            Path::new("c:/users/exemplo/appdata/local/com.synqo.falcao-router/accounts/ABC"),
            base
        ));
        assert!(!is_strictly_inside(base, base));
        assert!(!is_strictly_inside(
            Path::new(r"C:\Users\exemplo\AppData\Local\com.synqo.falcao-router\accounts-x\ABC"),
            base
        ));
        assert!(!is_strictly_inside(
            Path::new(r"C:\Users\exemplo\AppData\Local\com.synqo.falcao-router\accounts\..\..\x"),
            base
        ));
    }
}
