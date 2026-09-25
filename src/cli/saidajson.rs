//! A SAÍDA DE MÁQUINA (`--json`) — o contrato com a janela e com o hub.
//!
//! **Onde:** o `--json` de `accounts`, `detect`, `repos`, `status` e `log`.
//!
//! ## Escrito à MÃO, sem `derive` — e a razão é o leitor
//!
//! Este documento é lido por código que vive em **outro repositório**. Com `derive`, renomear um
//! campo do `Conta` mudaria o contrato sem aparecer em lugar nenhum do diff, e o outro lado
//! descobriria isso em produção lendo um campo que sumiu.
//!
//! Há um segundo motivo, específico deste app: o `Conta` **também** é serializado por `serde`
//! para o arquivo de cadastro em disco. Se o mesmo `derive` servisse aos dois, o formato em
//! disco e o contrato da janela ficariam amarrados — e renomear um campo por clareza interna
//! quebraria a janela, ou migrar o disco exigiria mexer na janela. São duas coisas com prazos
//! diferentes, e separá-las é o que permite mudar uma sem a outra.
//!
//! ## As chaves NUNCA são traduzidas, e não há prosa aqui
//!
//! Chave de JSON é decisão de máquina. Este projeto já pagou por ter parseado rótulo em
//! português numa janela, que devolvia vazio nos outros dezenove idiomas **sem erro nenhum**.
//!
//! **A origem de uma sugestão sai como SLUG, não como a frase.** O `Origem::descricao()` é prosa
//! para humano (*"do `gh auth status`"*) e muda de redação; o slug (`gh`) é o que a janela casa.

use git_casa::contas::{Auth, Conta};
use git_casa::deteccao::{Origem, Sugestao};
use git_casa::historico::{Commit, Upstream};
use git_casa::repos::{EstadoLocal, Remoto};

/// **O quê:** escapa uma string para dentro de JSON.
///
/// **Onde:** todo lugar deste arquivo. Nome de repositório, assunto de commit e nome de autor
/// vêm de FORA — do serviço e do `git log` — e podem trazer aspas, barra e controle. Um escape
/// incompleto produz um documento que o outro lado recusa, com uma mensagem que fala do parser
/// e nunca do texto que a causou.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// `Option<&str>` → `"texto"` ou `null`.
///
/// **`null` e `""` são coisas DIFERENTES:** um repositório sem remoto não é um repositório com
/// remoto vazio, e a janela desenha os dois casos de formas diferentes.
fn opt(v: Option<&String>) -> String {
    match v {
        Some(s) => format!("\"{}\"", esc(s)),
        None => "null".into(),
    }
}

/// **O quê:** o slug ESTÁVEL de como uma conta autentica.
///
/// **Onde:** [`conta`]. Conjunto fechado (`ssh` | `gh`) — a janela ramifica por ele.
fn auth_slug(a: &Auth) -> String {
    match a {
        Auth::Ssh { chave } => format!("{{\"tipo\":\"ssh\",\"chave\":\"{}\"}}", esc(chave)),
        Auth::Gh => "{\"tipo\":\"gh\",\"chave\":null}".into(),
    }
}

/// **O quê:** o slug ESTÁVEL de onde uma conta foi detectada.
///
/// **Onde:** [`deteccao`]. O `Origem::descricao()` é prosa para humano e muda de redação; isto
/// é o que a janela casa, e por isso não pode mudar sem aparecer no diff.
fn origem_slug(o: &Origem) -> &'static str {
    match o {
        Origem::Gh => "gh",
        Origem::GitGlobal => "git-global",
        Origem::SshConfig => "ssh-config",
        // O caminho do repositório NÃO entra no slug: ele é o CASO, não a identidade do caso.
        // A janela ramifica por "veio de um repo"; qual repo é prosa, e vai no campo separado.
        Origem::Repo(_) => "repo",
    }
}

/// **O quê:** o caminho do repositório de onde a sugestão veio, quando houver.
///
/// **Onde:** [`deteccao`]. Separado do slug porque são duas perguntas: *de que TIPO de fonte
/// veio* (conjunto fechado, a janela ramifica) e *de qual arquivo* (texto, a janela mostra).
fn origem_onde(o: &Origem) -> Option<String> {
    match o {
        Origem::Repo(p) => Some(p.to_string_lossy().into_owned()),
        _ => None,
    }
}

/// **O quê:** os campos de uma conta, SEM as chaves do objeto.
///
/// **Onde:** [`conta`] e [`contas`], que acrescentam campos próprios.
///
/// **Existe porque a primeira versão fazia cirurgia de string, e ela estava errada:** o
/// `contas` montava o objeto e depois tirava a chave final com `trim_end_matches('}')` para
/// acrescentar o `alias_ok`. Mas `trim_end_matches` remove **todas** as ocorrências finais — e
/// a conta termina em `}}`, porque o último campo é o objeto do `auth`. O documento saía
/// inválido, e o erro só aparecia num parser estrito.
///
/// Devolver os campos e deixar quem chama fechar o objeto acaba com a classe inteira.
fn campos_da_conta(c: &Conta) -> String {
    format!(
        "\"rotulo\":\"{}\",\"usuario\":\"{}\",\"email\":\"{}\",\"servico\":\"{}\",\"auth\":{}",
        esc(&c.rotulo),
        esc(&c.usuario),
        esc(&c.email),
        esc(&c.servico),
        auth_slug(&c.auth)
    )
}

/// Uma conta como objeto fechado.
fn conta(c: &Conta) -> String {
    format!("{{{}}}", campos_da_conta(c))
}

/// **O quê:** `accounts --json`.
///
/// **`alias_ok` entra aqui porque a janela precisa dele para desenhar o estado**, e calculá-lo
/// do lado dela exigiria reimplementar a leitura do `~/.ssh/config` — duas leis para o mesmo
/// fato, e a que divergisse mostraria "falta alias" sobre uma conta configurada.
pub fn contas(v: &[(Conta, bool)]) -> String {
    let itens: Vec<String> = v
        .iter()
        .map(|(c, alias_ok)| format!("{{{},\"alias_ok\":{}}}", campos_da_conta(c), alias_ok))
        .collect();
    format!("{{\"contas\":[{}]}}", itens.join(","))
}

/// **O quê:** `detect --json`.
pub fn deteccao(v: &[Sugestao]) -> String {
    let itens: Vec<String> = v
        .iter()
        .map(|s| {
            format!(
                "{{\"conta\":{},\"origem\":\"{}\",\"origem_onde\":{},\"ja_cadastrada\":{}}}",
                conta(&s.conta),
                origem_slug(&s.origem),
                opt(origem_onde(&s.origem).as_ref()),
                s.ja_cadastrada
            )
        })
        .collect();
    format!("{{\"sugestoes\":[{}]}}", itens.join(","))
}

/// **O quê:** `repos --json`. Agrupado por conta, porque é como a janela lista.
pub fn repositorios(v: &[(String, Result<Vec<Remoto>, String>)]) -> String {
    let itens: Vec<String> = v
        .iter()
        .map(|(rotulo, r)| match r {
            Ok(rs) => {
                let lista: Vec<String> = rs
                    .iter()
                    .map(|x| {
                        format!(
                            "{{\"caminho\":\"{}\",\"privado\":{},\"descricao\":\"{}\",\"atualizado\":\"{}\"}}",
                            esc(&x.caminho),
                            x.privado,
                            esc(&x.descricao),
                            esc(&x.atualizado)
                        )
                    })
                    .collect();
                format!(
                    "{{\"rotulo\":\"{}\",\"repos\":[{}],\"erro\":null}}",
                    esc(rotulo),
                    lista.join(",")
                )
            }
            // O ERRO POR CONTA, e não um erro global: uma conta sem `gh` logado não pode
            // apagar da tela os repositórios das outras. A janela desenha a linha com o motivo.
            Err(e) => {
                format!("{{\"rotulo\":\"{}\",\"repos\":[],\"erro\":\"{}\"}}", esc(rotulo), esc(e))
            }
        })
        .collect();
    format!("{{\"contas\":[{}]}}", itens.join(","))
}

/// **O quê:** `status --json` — o que ainda não saiu da máquina.
pub fn estado(v: &[EstadoLocal]) -> String {
    let itens: Vec<String> = v
        .iter()
        .map(|e| {
            format!(
                "{{\"nome\":\"{}\",\"raiz\":\"{}\",\"conta\":{},\"email\":\"{}\",\
                 \"remoto\":{},\"nao_enviados\":{},\"sujo\":{}}}",
                esc(&e.nome),
                esc(&e.raiz.to_string_lossy()),
                opt(e.conta.as_ref()),
                esc(&e.email),
                opt(e.remoto.as_ref()),
                e.nao_enviados,
                e.sujo
            )
        })
        .collect();
    format!("{{\"projetos\":[{}]}}", itens.join(","))
}

/// **O quê:** `log --json` — os commits e o upstream.
pub fn log(up: Option<&Upstream>, cs: &[Commit]) -> String {
    let upstream = match up {
        Some(u) => format!(
            "{{\"branch\":\"{}\",\"remote\":{},\"ahead\":{},\"behind\":{}}}",
            esc(&u.branch),
            opt(u.remote.as_ref()),
            u.ahead,
            u.behind
        ),
        None => "null".into(),
    };
    let itens: Vec<String> = cs
        .iter()
        .map(|c| {
            format!(
                "{{\"hash\":\"{}\",\"short\":\"{}\",\"author\":\"{}\",\"date\":\"{}\",\
                 \"subject\":\"{}\",\"pushed\":{}}}",
                esc(&c.hash),
                esc(&c.short),
                esc(&c.author),
                esc(&c.date),
                esc(&c.subject),
                c.pushed
            )
        })
        .collect();
    format!("{{\"upstream\":{},\"commits\":[{}]}}", upstream, itens.join(","))
}
