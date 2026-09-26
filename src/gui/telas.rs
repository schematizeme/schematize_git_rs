//! O que cada tela mostra — lido do `--json` do binário headless, nunca de saída humana.
//!
//! **O quê:** transforma os documentos de `status --json`, `accounts --json` e `repos --json`
//! nas linhas que a janela desenha.
//!
//! **Onde:** [`crate::gui`], que liga isto às propriedades do Slint.
//!
//! ## Funções PURAS, e a razão tem nome
//!
//! Tudo aqui entra texto e sai lista. O subprocesso fica em [`crate::gui::cli`]. Assim o
//! comportamento é afirmável com JSON inválido, truncado, hostil ou de outro idioma — sem
//! depender de haver um repositório git, uma conta cadastrada ou o `gh` logado na máquina de
//! quem roda a suíte.
//!
//! Este ecossistema já pagou duas vezes por não ter feito isso: a janela que casava **rótulo em
//! português** e devolvia vazio nos outros dezenove idiomas sem erro nenhum, e a pílula que saía
//! vazia contra um binário de outra versão.

use super::json::ler;

/// Um projeto e o que dele ainda não saiu desta máquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projeto {
    pub nome: String,
    pub raiz: String,
    /// O rótulo da conta aplicada, ou vazio quando o repositório não tem uma.
    pub conta: String,
    pub email: String,
    /// O remoto, ou vazio. **Vazio aqui é um ESTADO**, não um erro: repositório sem remoto é
    /// o caso mais grave desta tela, porque nada nele jamais saiu.
    pub remoto: String,
    pub nao_enviados: i64,
    pub sujo: bool,
}

impl Projeto {
    /// **O quê:** este projeto corre risco de perder trabalho com a máquina?
    ///
    /// **Onde:** a aba Projetos, para o destaque e para a contagem do topo.
    ///
    /// **Sem remoto conta como risco mesmo com zero commits não enviados**, e é o ponto: um
    /// repositório sem remoto não tem para onde enviar, então *tudo* nele só existe aqui. Medir
    /// só `nao_enviados > 0` diria "tudo certo" sobre o caso pior.
    pub fn em_risco(&self) -> bool {
        self.remoto.is_empty() || self.nao_enviados > 0
    }
}

/// Uma conta cadastrada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conta {
    pub rotulo: String,
    pub usuario: String,
    pub email: String,
    pub servico: String,
    /// O arquivo de chave em `~/.ssh`, ou vazio (aí a autenticação vai pelo `gh`).
    pub chave: String,
    /// O alias SSH desta conta já está no `~/.ssh/config`?
    pub alias_ok: bool,
}

/// Um repositório no serviço, dentro da conta que o lista.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub caminho: String,
    pub privado: bool,
    pub descricao: String,
    pub atualizado: String,
}

/// Os repositórios de UMA conta — ou o motivo de não ter dado para listá-los.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReposDaConta {
    pub rotulo: String,
    pub repos: Vec<Repo>,
    /// Vazio quando deu certo. **Por conta, e não global:** uma conta sem `gh` logado não pode
    /// apagar da tela os repositórios das outras.
    pub erro: String,
}

/// **O quê:** os projetos do `status --json`.
///
/// **Onde:** a aba Projetos.
///
/// **`Err` e lista vazia são coisas diferentes:** nenhum projeto em risco é `Ok(vec![])`, e a
/// tela diz isso. Um documento ilegível é `Err`, e a tela mostra o erro. Confundir os dois faria
/// a janela afirmar "nada a enviar" sobre uma falha de leitura — que é o pior que esta tela
/// pode dizer, porque é exatamente a frase que faz alguém formatar a máquina tranquilo.
pub fn ler_projetos(texto: &str) -> Result<Vec<Projeto>, String> {
    let doc = ler(texto)?;
    Ok(doc
        .arr("projetos")
        .iter()
        .map(|p| Projeto {
            nome: p.str_ou_vazio("nome"),
            raiz: p.str_ou_vazio("raiz"),
            conta: p.str_ou_vazio("conta"),
            email: p.str_ou_vazio("email"),
            remoto: p.str_ou_vazio("remoto"),
            nao_enviados: p.num("nao_enviados").unwrap_or(0.0) as i64,
            sujo: p.bool("sujo") == Some(true),
        })
        .collect())
}

/// **O quê:** as contas do `accounts --json`.
///
/// **Onde:** a aba Contas.
pub fn ler_contas(texto: &str) -> Result<Vec<Conta>, String> {
    let doc = ler(texto)?;
    Ok(doc
        .arr("contas")
        .iter()
        .map(|c| Conta {
            rotulo: c.str_ou_vazio("rotulo"),
            usuario: c.str_ou_vazio("usuario"),
            email: c.str_ou_vazio("email"),
            servico: c.str_ou_vazio("servico"),
            chave: c.str_ou_vazio("chave"),
            alias_ok: c.bool("alias_ok") == Some(true),
        })
        .collect())
}

/// **O quê:** os repositórios do `repos --json`, agrupados por conta.
///
/// **Onde:** a aba Repositórios.
pub fn ler_repos(texto: &str) -> Result<Vec<ReposDaConta>, String> {
    let doc = ler(texto)?;
    Ok(doc
        .arr("contas")
        .iter()
        .map(|c| ReposDaConta {
            rotulo: c.str_ou_vazio("rotulo"),
            repos: c
                .arr("repos")
                .iter()
                .map(|r| Repo {
                    caminho: r.str_ou_vazio("caminho"),
                    privado: r.bool("privado") == Some(true),
                    descricao: r.str_ou_vazio("descricao"),
                    atualizado: r.str_ou_vazio("atualizado"),
                })
                .collect(),
            erro: c.str_ou_vazio("erro"),
        })
        .collect())
}

/// **O quê:** quantas contas o `detect --json` encontrou e ainda NÃO estão cadastradas.
///
/// **Onde:** a aba Contas, quando ela está vazia.
///
/// **Existe porque "nenhuma conta cadastrada" era uma meia-verdade.** O app sabe ler conta do
/// `gh`, do git config, do `~/.ssh/config` e do e-mail dos repositórios — nesta máquina ele
/// achou três — e a tela vazia não dizia nada disso. Mandar a pessoa digitar do zero o que o
/// programa já leu é o §37.48 ao contrário: o software se adapta, não cobra.
///
/// **Só conta as NÃO cadastradas.** Somar as que já estão faria a frase prometer um trabalho
/// que já foi feito.
pub fn sugestoes_novas(texto: &str) -> usize {
    let Ok(doc) = ler(texto) else { return 0 };
    doc.arr("sugestoes").iter().filter(|s| s.bool("ja_cadastrada") != Some(true)).count()
}

/// **O quê:** a frase do topo da aba Projetos.
///
/// **Onde:** a aba Projetos.
///
/// **Ela conta PROJETOS em risco, não commits.** "12 commits não enviados" num projeto só é uma
/// notícia menor do que "3 projetos podem sumir com a máquina"; o que se perde é o projeto.
pub fn resumo_projetos(ps: &[Projeto]) -> String {
    let risco = ps.iter().filter(|p| p.em_risco()).count();
    if risco == 0 {
        return format!("{} projeto(s) · tudo já saiu daqui", ps.len());
    }
    format!("{} projeto(s) · {risco} com trabalho que só existe nesta máquina", ps.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJ: &str = r#"{"projetos":[
      {"nome":"loja","raiz":"/h/loja","conta":"pessoal","email":"a@b.c",
       "remoto":"git@github.com:u/loja.git","nao_enviados":3,"sujo":true},
      {"nome":"rascunho","raiz":"/h/rascunho","conta":null,"email":"a@b.c",
       "remoto":null,"nao_enviados":0,"sujo":false},
      {"nome":"ok","raiz":"/h/ok","conta":"trabalho","email":"x@y.z",
       "remoto":"git@github.com:u/ok.git","nao_enviados":0,"sujo":false}
    ]}"#;

    #[test]
    fn le_os_projetos() {
        let p = ler_projetos(PROJ).expect("lê");
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].nome, "loja");
        assert_eq!(p[0].nao_enviados, 3);
        assert!(p[0].sujo);
        assert_eq!(p[1].remoto, "", "`null` vira vazio, e vazio é um estado");
        assert_eq!(p[1].conta, "");
    }

    /// **Sem remoto é risco MESMO com zero não-enviados.**
    ///
    /// Um repositório sem remoto não tem para onde enviar: tudo nele só existe aqui. Medir só
    /// `nao_enviados > 0` diria "tudo certo" sobre o caso pior desta tela.
    #[test]
    fn sem_remoto_conta_como_risco() {
        let p = ler_projetos(PROJ).expect("lê");
        assert!(p[0].em_risco(), "3 commits não enviados");
        assert!(p[1].em_risco(), "sem remoto, e é o caso GRAVE");
        assert!(!p[2].em_risco(), "com remoto e nada pendente");
    }

    /// O resumo conta PROJETOS em risco — o que se perde é o projeto, não o commit.
    #[test]
    fn o_resumo_conta_projetos_e_nao_commits() {
        let p = ler_projetos(PROJ).expect("lê");
        let r = resumo_projetos(&p);
        assert!(r.contains("3 projeto(s)"), "{r}");
        assert!(r.contains("2 com trabalho"), "{r}");
        // E a frase NÃO conta commits: os 3 não-enviados do primeiro projeto não podem
        // aparecer como se fossem a medida — o que se perde é o projeto, não o commit.
        assert!(!r.contains("commit"), "o resumo mede projetos: {r}");
        // Nada em risco tem frase PRÓPRIA: "0 com trabalho que só existe aqui" faria a pessoa
        // procurar um problema que não existe.
        let vazio = resumo_projetos(&[]);
        assert!(vazio.contains("tudo já saiu daqui"), "{vazio}");
    }

    /// **Vazio e ERRO são estados diferentes** — e aqui a confusão é a mais cara do app: a
    /// janela diria "nada a enviar" sobre uma falha de leitura, que é a frase que faz alguém
    /// formatar a máquina tranquilo.
    #[test]
    fn vazio_e_erro_nao_se_confundem() {
        assert_eq!(ler_projetos(r#"{"projetos":[]}"#).expect("vazio é Ok"), vec![]);
        assert!(ler_projetos("{ isto nao e json").is_err(), "ilegível é Err, não lista vazia");
        assert_eq!(ler_projetos("{}").expect("objeto vazio"), vec![]);
    }

    #[test]
    fn le_as_contas_com_o_alias() {
        let c = ler_contas(
            r#"{"contas":[{"rotulo":"pessoal","usuario":"u","email":"a@b.c",
                "servico":"github","chave":"id_ed25519","alias_ok":true},
               {"rotulo":"trabalho","usuario":"v","email":"x@y.z",
                "servico":"github","chave":"","alias_ok":false}]}"#,
        )
        .expect("lê");
        assert_eq!(c.len(), 2);
        assert!(c[0].alias_ok);
        assert!(!c[1].alias_ok, "sem alias é o estado que a tela precisa mostrar");
        assert_eq!(c[1].chave, "", "chave vazia = autentica pelo gh");
    }

    /// **O erro é POR CONTA.** Uma conta sem `gh` logado não pode apagar da tela os
    /// repositórios das outras — e a lista dela sai vazia COM motivo, não vazia e calada.
    #[test]
    fn o_erro_de_uma_conta_nao_apaga_as_outras() {
        let r = ler_repos(
            r#"{"contas":[
              {"rotulo":"pessoal","repos":[{"caminho":"u/loja","privado":true,
                "descricao":"a loja","atualizado":"2026-09-01"}],"erro":null},
              {"rotulo":"trabalho","repos":[],"erro":"gh não está logado nesta conta"}
            ]}"#,
        )
        .expect("lê");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].repos.len(), 1);
        assert_eq!(r[0].erro, "");
        assert!(r[0].repos[0].privado);
        assert!(r[1].repos.is_empty());
        assert_eq!(r[1].erro, "gh não está logado nesta conta", "o motivo chega à tela");
    }

    /// **A tela vazia diz o que o app JÁ SABE.** Só as ainda não cadastradas entram na conta:
    /// somar as que já estão prometeria um trabalho que já foi feito.
    #[test]
    fn a_contagem_de_sugestoes_ignora_o_que_ja_esta_cadastrado() {
        let doc = r#"{"sugestoes":[
          {"conta":{"rotulo":"a"},"origem":"gh","ja_cadastrada":false},
          {"conta":{"rotulo":"b"},"origem":"repo","ja_cadastrada":true},
          {"conta":{"rotulo":"c"},"origem":"ssh-config","ja_cadastrada":false}
        ]}"#;
        assert_eq!(sugestoes_novas(doc), 2);
        // Documento ilegível vira ZERO, e não um número inventado: a frase some em vez de
        // mentir. Aqui o silêncio é honesto — a lista de contas já mostrou o erro dela.
        assert_eq!(sugestoes_novas("{ nao e json"), 0);
        assert_eq!(sugestoes_novas(r#"{"sugestoes":[]}"#), 0);
    }

    /// Entrada hostil não panica — janela que morre ao abrir é pior que lista vazia.
    #[test]
    fn entrada_hostil_nao_panica() {
        let fundo = format!("{}{}", "[".repeat(3000), "]".repeat(3000));
        for lixo in [
            "",
            "null",
            "[]",
            "0",
            "\"txt\"",
            "\u{0}",
            &fundo,
            r#"{"projetos":"nao e lista"}"#,
            r#"{"projetos":[null]}"#,
            r#"{"projetos":[{"nome":42,"nao_enviados":"muitos","sujo":7}]}"#,
            r#"{"contas":[{"alias_ok":"talvez"}]}"#,
        ] {
            let _ = ler_projetos(lixo);
            let _ = ler_contas(lixo);
            let _ = ler_repos(lixo);
            let _ = sugestoes_novas(lixo);
        }
    }

    /// **Campo de tipo errado não vira afirmação.** `nao_enviados: "muitos"` não pode virar um
    /// número inventado, e `sujo: 7` não pode virar `true` — a tela estaria mentindo sobre o
    /// estado do repositório de alguém.
    #[test]
    fn tipo_errado_nao_vira_afirmacao() {
        let p = ler_projetos(
            r#"{"projetos":[{"nome":"x","nao_enviados":"muitos","sujo":7,"remoto":"r"}]}"#,
        )
        .expect("lê");
        assert_eq!(p[0].nao_enviados, 0, "texto não vira número");
        assert!(!p[0].sujo, "7 não vira `true`");
        assert!(!p[0].em_risco(), "com remoto e nada afirmado, não há risco a declarar");
    }
}
