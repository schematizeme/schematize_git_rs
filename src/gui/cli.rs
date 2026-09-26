//! A CASCA — falar com o binário `schematize-git`, e nada de domínio.
//!
//! **O quê:** resolve onde o binário headless está e roda um subcomando lendo o `--json`.
//!
//! **Onde:** as duas telas, que só desenham o que sai daqui.
//!
//! ## Isto é PLATAFORMA, e é o que torna a cópia legítima (D4)
//!
//! Este arquivo é irmão do `gestor.rs` da janela do market. A duplicação é deliberada e
//! deliberadamente **burra**: nada aqui sabe o que é conta, commit, remoto ou alias. O que a
//! casca sabe do app entra por `--json`, e sai como texto.
//!
//! **Se um dia algo de domínio migrar para cá, o corte foi feito errado.** É a regra do D4, e
//! ela é verificável: procure neste arquivo uma palavra do negócio. Não há nenhuma.
//!
//! ## Por que a janela NÃO depende do crate `git_casa`
//!
//! Depender dele a faria compilar junto com o app e embutir a versão via git-dep — que é
//! exatamente o que fez a janela irmã "abrir a versão antiga". Falando só com o binário, ela
//! sempre conversa com o que está instalado, não com o que estava quando ela foi compilada.

use std::path::PathBuf;
use std::process::{Command, Stdio};

/// **O quê:** o nome do binário headless nesta plataforma.
///
/// **Onde:** [`bin`]. Constante e não literal espalhado: nome de binário que não bate com o
/// dono já deixou o update de um app da casa morto por um release inteiro, e **nada dá erro**
/// nesse caso — o comando simplesmente não é encontrado, e a janela mostra um erro sobre
/// outra coisa.
fn nome() -> &'static str {
    if cfg!(windows) {
        "schematize-git.exe"
    } else {
        "schematize-git"
    }
}

/// **O quê:** o caminho do binário headless: ao lado deste executável → `~/.cargo/bin` → PATH.
///
/// **Onde:** [`ler_json`] e [`comando_para_terminal`].
///
/// **A ordem importa, e o "ao lado" vem primeiro** porque é onde o `install.sh` põe o par, e é
/// o par que se atualiza junto. Depois `~/.cargo/bin`, e só então o PATH — que a janela aberta
/// pelo lançador do desktop recebe **mínimo**, sem `~/.cargo/bin`. Sem essa cadeia, a janela
/// aberta pelo ícone não acharia o binário que está instalado.
pub fn bin() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let c = dir.join(nome());
            if c.is_file() {
                return c;
            }
        }
    }
    if let Some(home) = home() {
        let c = home.join(".cargo").join("bin").join(nome());
        if c.is_file() {
            return c;
        }
    }
    PathBuf::from(nome())
}

/// **O quê:** o diretório home, na variável que cada plataforma usa.
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

/// **O quê:** roda `<bin> <args…>` — EXATAMENTE o que recebe — e devolve o stdout, ou POR QUE
/// não deu.
///
/// **A casca NÃO acrescenta flag nenhuma, e isto foi aprendido errando.** A primeira versão
/// chamava-se `ler_json` e colava um `--json` no fim, o que parecia conveniente e quebrou os
/// quatro botões da janela de uma vez: nos que já pediam `--json`, o clap recusou a flag
/// repetida; no `sql`, que não aceita `--json`, ele recusou a flag inteira. **Casca que
/// adivinha o comando decide domínio** — e decidir domínio é o que o D4 proíbe a ela.
///
/// **Onde:** as duas telas, ao abrir e ao recarregar.
///
/// **Devolve `Result`, e nunca uma string vazia**, porque os dois modos de falha — não
/// consegui executar, executei e o comando falhou — precisam chegar à tela como MOTIVO. Uma
/// string vazia viraria uma tela em branco, e tela em branco é a janela afirmando que não há
/// nada, quando o que houve foi não ter conseguido perguntar. É o defeito que a janela irmã
/// teve, e que custou uma versão inteira.
pub fn rodar(args: &[&str]) -> Result<String, String> {
    let b = bin();
    let out = Command::new(&b)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("não consegui executar {}: {e}", b.display()))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.trim();
        return Err(format!(
            "{} {} falhou ({}){}",
            b.display(),
            args.join(" "),
            out.status,
            if err.is_empty() { String::new() } else { format!(": {err}") }
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// **O quê:** monta o comando de shell que roda um subcomando NO TERMINAL, com o eco no topo.
///
/// **Onde:** as ações que mudam a máquina, que abrem terminal em vez de agir na janela (D6).
///
/// **O `bin` tem de ser o caminho ABSOLUTO, e este é um bug já visto em uso.** A janela aberta
/// pelo lançador do desktop tem PATH mínimo, e o terminal que ela abre **herda** esse PATH.
/// Com o nome puro, a pessoa clica e recebe `comando não encontrado` sobre um binário que está
/// instalado.
///
/// **O eco mostra o MESMO comando que roda.** Se ele dissesse uma coisa e executasse outra, o
/// erro que a pessoa copiar para pedir ajuda seria sobre um comando que ninguém rodou.
///
/// **O `read` no fim** segura o terminal depois que o comando termina — sem ele, a janela
/// fecha junto com o processo e o erro some com ela.
pub fn comando_para_terminal(bin: &str, args: &[&str]) -> String {
    let linha = format!("{bin} {}", args.join(" "));
    format!(
        "echo '── {linha} ──'; echo; \
         {linha}; \
         echo; read -n1 -s -r -p '…'"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O nome do binário conhece a plataforma, e é o do app CERTO. Apontar para outro nome não
    /// dá erro — dá "comando não encontrado" sobre o app errado, que é pior.
    #[test]
    fn o_nome_do_binario_conhece_a_plataforma_e_o_dono() {
        let n = nome();
        assert!(n.starts_with("schematize-git"), "{n}");
        if cfg!(windows) {
            assert!(n.ends_with(".exe"), "{n}");
        } else {
            assert!(!n.ends_with(".exe"), "{n}");
        }
    }

    /// `bin()` nunca devolve vazio — devolveria um comando que começa com espaço e tentaria
    /// executar o primeiro argumento como programa.
    #[test]
    fn o_caminho_nunca_e_vazio() {
        let b = bin();
        assert!(!b.as_os_str().is_empty());
        assert!(b.to_string_lossy().contains("schematize-git"));
    }

    /// **O COMANDO USA O CAMINHO ABSOLUTO**, e o eco bate com o que roda. É o bug de PATH
    /// mínimo, já visto em uso na janela irmã.
    #[test]
    fn o_comando_usa_caminho_absoluto_e_o_eco_bate() {
        let g = "/home/u/.cargo/bin/schematize-git";
        let c = comando_para_terminal(g, &["use", "pessoal"]);
        assert_eq!(c.matches(&format!("{g} use pessoal")).count(), 2, "eco e execução: {c}");
        // Self-check: com o nome puro a asserção acima não provaria nada sobre o caminho.
        let ruim = comando_para_terminal("schematize-git", &["use"]);
        assert!(!ruim.contains("/home/u/.cargo/bin/"), "o self-check parou de valer");
    }

    /// O terminal espera uma tecla — sem isso ele fecha junto com o processo e o erro que a
    /// pessoa precisa ler some com ele.
    #[test]
    fn o_terminal_nao_fecha_na_cara() {
        assert!(comando_para_terminal("/b/x", &["use"]).contains("read -n1"));
    }

    /// **A casca não sabe NADA do domínio (D4).** Este teste lê o próprio arquivo e procura as
    /// palavras do negócio. É a regra do D4 virando verificação em vez de recomendação: se um
    /// dia algo de domínio migrar para cá, o corte foi feito errado, e isto reprova.
    #[test]
    fn a_casca_nao_sabe_nada_do_dominio() {
        let fonte = include_str!("cli.rs");
        // Só o código de PRODUÇÃO, e dentro dele só o que não é comentário.
        //
        // As duas exclusões têm razões diferentes. Os testes ficam de fora porque este mesmo
        // teste precisa NOMEAR as palavras do domínio para procurá-las. Os comentários ficam
        // de fora porque a primeira versão disto reprovou na própria frase do cabeçalho que
        // diz "nada aqui sabe o que é limite, slice, GTT ou serviço" — explicar a ausência
        // exige nomear o que está ausente, e um teste que proíbe isso proíbe a documentação.
        //
        // O que sobra é o que importa: nenhuma FUNÇÃO desta casca pode saber do negócio.
        let producao = fonte.split("#[cfg(test)]").next().unwrap();
        let codigo: String = producao
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        // As palavras são as DESTE domínio, e não as do app de onde esta casca foi portada.
        // Herdar a lista do optimizer (`slice`, `cgroup`, `vram`…) faria o teste passar sempre
        // aqui — um guard que procura o que nunca existiria neste repo não mede nada.
        for palavra in ["commit", "upstream", "remoto", "alias", "conta", "push", "repositorio"] {
            assert!(
                !codigo.contains(palavra),
                "a casca é PLATAFORMA (D4): `{palavra}` é domínio e não pode estar no código daqui"
            );
        }
        // Self-check: o varredor precisa saber ACHAR. Sem isto, um filtro que comesse o
        // arquivo inteiro faria o teste passar sempre — e um guard que nunca reprova é cego.
        assert!(codigo.contains("comando_para_terminal"), "o filtro comeu o código de produção");
    }
}
