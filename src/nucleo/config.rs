//! Os diretórios de desenvolvimento, lidos da config COMPARTILHADA do ecossistema.
//!
//! **O quê:** `dev_dirs` e `recent_projects` de `~/.claude/schematize/config.json`.
//!
//! **Onde:** `git status` (varre os projetos) e `git detect` (procura identidade por repo).
//!
//! ## SOMENTE LEITURA, e isso é o desenho
//!
//! Este arquivo é escrito pelo hub — é lá que a pessoa cadastra um diretório de dev, pela
//! janela. Aqui ele é apenas **lido**, e nenhuma função deste módulo grava.
//!
//! A assimetria é deliberada. Um segundo escritor no mesmo JSON traria de volta exatamente o
//! problema que o `caixa.rs` do overdev documenta: ciclo ler-modificar-escrever, e quem grava
//! por último apaga o outro **sem erro e sem aviso**. Com um dono só, não há disputa.
//!
//! ## Por que ler o arquivo em vez de exigir `--dir`
//!
//! O `--dir` existe e ganha quando é passado. Mas exigi-lo faria a pessoa repetir, em todo
//! comando, uma lista que ela já cadastrou uma vez — e o piso de UX da casa (§37.48) diz que
//! software que obriga o usuário a saber de internals é bug do software.
//!
//! **A config ausente NÃO é erro**, e é o piso 10 em ação: sem o hub instalado, o arquivo não
//! existe, e este app continua funcionando com `--dir` ou com o diretório atual.

use serde::Deserialize;

/// Só os dois campos que interessam.
///
/// **`#[serde(default)]` em todos, e o resto do arquivo é IGNORADO de propósito:** a config é do
/// ecossistema e tem campos de outros apps. Recusar o documento por um campo desconhecido faria
/// este app quebrar toda vez que o hub ganhasse uma preferência nova.
#[derive(Debug, Default, Deserialize)]
struct Config {
    #[serde(default)]
    dev_dirs: Vec<String>,
    #[serde(default)]
    recent_projects: Vec<String>,
}

/// **O quê:** lê a config, ou o default quando ela não existe nem é legível.
///
/// **Onde:** [`dev_dirs`] e [`recent_projects`].
fn carregar() -> Config {
    let p = super::util::dados_dir().join("config.json");
    std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// **O quê:** os diretórios de desenvolvimento cadastrados.
///
/// **Onde:** `git status`, quando nenhum `--dir` foi passado.
pub fn dev_dirs() -> Vec<String> {
    carregar().dev_dirs
}

/// **O quê:** os projetos abertos recentemente.
///
/// **Onde:** `git detect`, para achar identidade configurada por repositório — e-mail LOCAL
/// diferente do global é o sinal de que há mais de uma pessoa nesta máquina.
pub fn recent_projects() -> Vec<String> {
    carregar().recent_projects
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Campo desconhecido não derruba a leitura.** A config é do ecossistema, e o hub ganha
    /// preferências novas sem avisar este app.
    #[test]
    fn campo_de_outro_app_e_ignorado() {
        let c: Config = serde_json::from_str(
            r#"{"dev_dirs":["/a"],"recent_projects":["/b"],"tema":"escuro","idioma":"pt"}"#,
        )
        .expect("campo extra não pode reprovar o documento");
        assert_eq!(c.dev_dirs, ["/a"]);
        assert_eq!(c.recent_projects, ["/b"]);
    }

    /// Campo ausente vira lista vazia, não erro — o piso 10: sem o hub, este app funciona.
    #[test]
    fn campo_ausente_vira_lista_vazia() {
        let c: Config = serde_json::from_str("{}").expect("documento vazio é válido");
        assert!(c.dev_dirs.is_empty() && c.recent_projects.is_empty());
    }

    /// Documento ilegível cai no default em vez de panicar — a config é editável à mão.
    #[test]
    fn documento_ilegivel_nao_panica() {
        for lixo in ["", "{ isto nao e json", "[]", "null", "0"] {
            let _: Config = serde_json::from_str(lixo).unwrap_or_default();
        }
    }
}
