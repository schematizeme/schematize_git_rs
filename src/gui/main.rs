//! **schematize-git-gui** — a janela do `schematize-git`.
//!
//! **O quê:** mostra o que ainda não saiu desta máquina, as contas cadastradas e os
//! repositórios de cada uma; e manda para o TERMINAL tudo que muda a máquina.
//!
//! **Onde:** o ícone do app, e a aba Git do hub, que delega para cá.
//!
//! ## Ela mora no MESMO repo do CLI (ADR-0020), e continua falando por `--json`
//!
//! **Estar no mesmo repo é conveniência de distribuição, não permissão para acoplar.** A janela
//! não usa o crate `git_casa`: ela roda o binário e lê o `--json`, como as irmãs. Há teste
//! lendo o próprio fonte para cobrar isso.
//!
//! ## Por que NENHUMA ação de escrita acontece aqui (D6)
//!
//! Aplicar identidade reescreve o `.git/config` de um repositório; escrever alias mexe no
//! `~/.ssh/config`; cadastrar conta pergunta dados. As três abrem TERMINAL, e por duas razões
//! que se somam: a pessoa VÊ o comando que vai rodar antes de ele rodar, e o `git`/`ssh`
//! herdam um TTY de verdade quando precisarem perguntar alguma coisa. Uma janela Slint não tem
//! como responder a um prompt de credencial — ela ficaria pendurada, e o erro morreria com ela.

mod cli;
mod json;
mod telas;
mod terminal;

slint::include_modules!();

use slint::{Model, ModelRc, SharedString, VecModel};

/// **O quê:** converte os projetos do domínio nas linhas que o Slint desenha.
///
/// **Onde:** [`recarregar`].
///
/// **O `risco` vem calculado do Rust**, e não de uma expressão no `.slint`: a regra "sem remoto
/// também é risco" tem teste do lado de cá, e reimplementá-la lá criaria duas leis para o mesmo
/// fato — a que divergisse diria "tudo certo" sobre o caso pior.
fn linhas_projetos(ps: &[telas::Projeto]) -> ModelRc<ProjetoUI> {
    ModelRc::new(VecModel::from(
        ps.iter()
            .map(|p| ProjetoUI {
                nome: p.nome.clone().into(),
                raiz: p.raiz.clone().into(),
                conta: p.conta.clone().into(),
                remoto: p.remoto.clone().into(),
                nao_enviados: p.nao_enviados as i32,
                sujo: p.sujo,
                risco: p.em_risco(),
            })
            .collect::<Vec<_>>(),
    ))
}

/// **O quê:** relê projetos e contas, e escreve as propriedades da tela.
///
/// **Onde:** a abertura da janela e o botão «Reler».
///
/// **São DOIS documentos, e a falha de um não apaga o outro.** Perder a lista de contas porque
/// o `status` falhou seria trocar informação por nada.
fn recarregar(w: &MainWindow, dirs: &[String]) {
    // `--dir` repassado, e não ignorado. Sem isto a janela mostra "nenhum projeto git nos
    // diretórios de dev" e a mensagem do CLI manda cadastrar um **pela janela do hub** — que é
    // justamente a tela que esta aqui substitui. Um beco circular, e o §37.48 chama isso de
    // bug do software: quem abriu a janela apontando para uma pasta já disse onde olhar.
    let mut args: Vec<&str> = vec!["status"];
    for d in dirs {
        args.push("--dir");
        args.push(d);
    }
    args.push("--json");
    match cli::rodar(&args).and_then(|t| telas::ler_projetos(&t)) {
        Ok(ps) => {
            w.set_resumo(telas::resumo_projetos(&ps).into());
            w.set_projetos(linhas_projetos(&ps));
            w.set_erro(SharedString::new());
        }
        Err(e) => {
            // Lista VAZIA com o erro na tela, e nunca lista vazia calada: "nada a enviar" sobre
            // uma falha de leitura é a frase que faz alguém formatar a máquina tranquilo.
            w.set_projetos(ModelRc::new(VecModel::from(Vec::<ProjetoUI>::new())));
            w.set_resumo(SharedString::new());
            w.set_erro(e.into());
        }
    }

    let mut vazia = true;
    match cli::rodar(&["accounts", "--json"]).and_then(|t| telas::ler_contas(&t)) {
        Ok(cs) => {
            vazia = cs.is_empty();
            w.set_contas(ModelRc::new(VecModel::from(
                cs.iter()
                    .map(|c| ContaUI {
                        rotulo: c.rotulo.clone().into(),
                        usuario: c.usuario.clone().into(),
                        email: c.email.clone().into(),
                        servico: c.servico.clone().into(),
                        chave: c.chave.clone().into(),
                        alias_ok: c.alias_ok,
                    })
                    .collect::<Vec<_>>(),
            )));
        }
        Err(e) => {
            if w.get_erro().is_empty() {
                w.set_erro(e.into());
            }
        }
    }

    // **Com a lista de contas VAZIA, a tela diz o que o app já sabe.** O `detect` lê conta do
    // `gh`, do git config, do `~/.ssh/config` e do e-mail dos repositórios; deixar a tela dizer
    // só "nenhuma conta cadastrada" seria esconder isso e mandar a pessoa digitar do zero.
    //
    // Só roda quando está vazia: com contas na tela, a varredura seria trabalho que ninguém
    // pediu, toda vez que alguém clica em «Reler».
    let n = if vazia {
        cli::rodar(&["detect", "--json"]).map(|t| telas::sugestoes_novas(&t)).unwrap_or(0)
    } else {
        0
    };
    w.set_sugestoes(n as i32);
}

/// **O quê:** lê os repositórios de cada conta pelo `gh` e escreve na aba.
///
/// **Onde:** o botão «Repositórios» e o «Reler» dela.
///
/// **Não roda ao abrir, e é de propósito:** esta é a única leitura que sai para a REDE, e pode
/// demorar ou pedir login. Fazê-la no arranque deixaria a janela parada antes de mostrar
/// qualquer coisa — inclusive as duas abas que já estavam prontas.
fn carregar_repos(w: &MainWindow) {
    // **Sem conta cadastrada, nem pergunta.** O `repos` falha com "nenhuma conta cadastrada —
    // `schematize-git add` ou `detect`", e essa frase é de TERMINAL: ela manda digitar um
    // comando para quem está numa janela que tem a aba do lado. Rodar um comando que se sabe
    // que vai falhar, para despejar o erro dele numa caixa amarela, é §37.48 — o software
    // sabia, e passou a conta para quem clicou.
    if w.get_contas().row_count() == 0 {
        w.set_repos(ModelRc::new(VecModel::from(Vec::<ContaReposUI>::new())));
        w.set_erro(SharedString::new());
        w.set_msg(
            "Nenhuma conta cadastrada — os repositórios vêm do `gh` de cada conta. \
             Comece pela aba Contas."
                .into(),
        );
        return;
    }
    w.set_msg(SharedString::new());
    w.set_carregando(true);
    let r = cli::rodar(&["repos", "--json"]).and_then(|t| telas::ler_repos(&t));
    w.set_carregando(false);
    match r {
        Ok(gs) => {
            w.set_repos(ModelRc::new(VecModel::from(
                gs.iter()
                    .map(|g| ContaReposUI {
                        rotulo: g.rotulo.clone().into(),
                        erro: g.erro.clone().into(),
                        repos: ModelRc::new(VecModel::from(
                            g.repos
                                .iter()
                                .map(|r| RepoUI {
                                    caminho: r.caminho.clone().into(),
                                    privado: r.privado,
                                    descricao: r.descricao.clone().into(),
                                    atualizado: r.atualizado.clone().into(),
                                })
                                .collect::<Vec<_>>(),
                        )),
                    })
                    .collect::<Vec<_>>(),
            )));
            w.set_erro(SharedString::new());
        }
        Err(e) => w.set_erro(e.into()),
    }
}

/// **O quê:** abre um subcomando no TERMINAL e diz na tela o que aconteceu.
///
/// **Onde:** as três ações que mudam a máquina (D6).
///
/// **Quando não há terminal, a mensagem traz o COMANDO** — quem não tem um emulador instalado
/// ainda consegue fazer a coisa, colando. Um "não consegui abrir o terminal" seco seria um
/// beco sem saída (§37.48).
fn no_terminal(w: &MainWindow, args: &[&str]) {
    let bin = cli::bin().display().to_string();
    let cmd = cli::comando_para_terminal(&bin, args);
    if terminal::abrir(&cmd) {
        w.set_msg("terminal aberto — releia esta tela quando ele terminar.".into());
    } else {
        w.set_msg(format!("não achei um terminal. Rode: {cmd}").into());
    }
}

fn main() -> Result<(), slint::PlatformError> {
    // **RESPONDE `--version` E SAI, antes de qualquer coisa gráfica.**
    //
    // Sem isto a janela IGNORA a flag e ABRE — e quem perguntou fica esperando. O `debugreport`
    // do hub pergunta a versão de cada binário do ecossistema, e as janelas irmãs tinham
    // exatamente este defeito.
    if std::env::args().skip(1).any(|a| a == "--version" || a == "-V") {
        println!("schematize-git-gui {}", git_casa::nucleo::procedencia::rotulo_versao());
        return Ok(());
    }

    let w = MainWindow::new()?;
    w.set_versao(
        format!("schematize-git-gui {}", git_casa::nucleo::procedencia::rotulo_versao()).into(),
    );
    let dirs = dirs_dos_args(&std::env::args().skip(1).collect::<Vec<_>>());
    recarregar(&w, &dirs);
    // A aba pedida vem DEPOIS da leitura, pela mesma razão da janela do database.
    w.set_aba(aba_dos_args(&std::env::args().skip(1).collect::<Vec<_>>()));
    if w.get_aba() == 2 {
        carregar_repos(&w);
    }

    {
        let weak = w.as_weak();
        let dirs = dirs.clone();
        w.on_recarregar(move || {
            if let Some(w) = weak.upgrade() {
                recarregar(&w, &dirs);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_carregar_repos(move || {
            if let Some(w) = weak.upgrade() {
                carregar_repos(&w);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_usar_conta(move |rotulo| {
            if let Some(w) = weak.upgrade() {
                no_terminal(&w, &["use", rotulo.as_str()]);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_escrever_alias(move |rotulo| {
            if let Some(w) = weak.upgrade() {
                no_terminal(&w, &["ssh-config", rotulo.as_str()]);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_cadastrar(move || {
            if let Some(w) = weak.upgrade() {
                // `detect` e não `add`: ele MOSTRA o que já existe nesta máquina antes de
                // pedir qualquer coisa. Mandar a pessoa digitar do zero o que o app já sabe
                // ler do `gh`, do git config e do `~/.ssh` é o §37.48 ao contrário.
                no_terminal(&w, &["detect"]);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_alternar_tema(move || {
            if let Some(w) = weak.upgrade() {
                w.set_dark(!w.get_dark());
            }
        });
    }

    w.run()
}

/// **O quê:** os diretórios pedidos em `--dir <caminho>`, repetível.
///
/// **Onde:** [`main`], e é por aqui que o hub passa os diretórios de dev que ele já conhece.
///
/// **Repetível porque o CLI aceita repetido**, e porque um workspace de microserviços é
/// exatamente o caso em que os projetos moram em mais de uma raiz. Função PURA.
///
/// **Valor que começa com `-` é a flag seguinte**, não o valor desta: sem essa guarda,
/// `--dir --aba contas` viraria um diretório chamado `--aba`.
fn dirs_dos_args(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--dir" {
            match args.get(i + 1) {
                Some(v) if !v.is_empty() && !v.starts_with('-') => {
                    out.push(v.clone());
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        i += 1;
    }
    out
}

/// **O quê:** a aba pedida em `--aba <nome>`: `0` Projetos, `1` Contas, `2` Repositórios.
///
/// **Onde:** [`main`], e é por aqui que **o hub delega a aba** sem perder em qual delas a
/// pessoa estava.
///
/// **Pelo NOME, e nome desconhecido cai em Projetos.** O número é detalhe interno do `.slint`,
/// e abrir numa aba inventada seria obedecer a um comando que ninguém entendeu. Função PURA.
fn aba_dos_args(args: &[String]) -> i32 {
    let Some(i) = args.iter().position(|a| a == "--aba") else { return 0 };
    match args.get(i + 1).map(String::as_str) {
        Some("contas") => 1,
        Some("repos") | Some("repositorios") => 2,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{aba_dos_args, dirs_dos_args};

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    /// **A aba pedida é a que abre, e nome desconhecido cai em Projetos.**
    ///
    /// É por aqui que o hub delega. Sem isto, a delegação entregaria sempre a primeira aba e a
    /// pessoa teria de achar a sua de novo.
    #[test]
    fn a_aba_pedida_abre_pelo_nome() {
        assert_eq!(aba_dos_args(&v(&["--aba", "contas"])), 1);
        assert_eq!(aba_dos_args(&v(&["--aba", "repos"])), 2);
        assert_eq!(aba_dos_args(&v(&["--aba", "repositorios"])), 2, "o apelido longo vale");
        assert_eq!(aba_dos_args(&v(&["--aba", "projetos"])), 0);
        assert_eq!(aba_dos_args(&v(&[])), 0, "sem pedido, a aba de partida");
        // Nome inventado NÃO abre aba inventada, e `--aba` no fim da linha não panica:
        // §37.48, invocação não prevista é bug do software.
        assert_eq!(aba_dos_args(&v(&["--aba", "turbinada"])), 0);
        assert_eq!(aba_dos_args(&v(&["--aba"])), 0);
    }

    /// **`--dir` é repassado, e é repetível.**
    ///
    /// Sem ele a janela mostra "nenhum projeto git nos diretórios de dev", e a mensagem do CLI
    /// manda cadastrar um **pela janela do hub** — que é a tela que esta substitui. Um beco
    /// circular: quem abriu a janela apontando para uma pasta já disse onde olhar.
    #[test]
    fn os_diretorios_pedidos_sao_repassados() {
        assert_eq!(dirs_dos_args(&v(&["--dir", "/a"])), ["/a"]);
        assert_eq!(dirs_dos_args(&v(&["--dir", "/a", "--dir", "/b"])), ["/a", "/b"], "repetível");
        assert!(dirs_dos_args(&v(&[])).is_empty(), "sem pedido, o CLI usa os dirs cadastrados");
        // Flag no fim da linha não panica, e o valor não engole a flag seguinte.
        assert!(dirs_dos_args(&v(&["--dir"])).is_empty());
        let a = v(&["--dir", "--aba", "contas"]);
        assert!(dirs_dos_args(&a).is_empty(), "`--aba` não é nome de diretório");
        assert_eq!(aba_dos_args(&a), 1, "e a flag seguinte continua sendo lida");
    }

    /// **A aba Repositórios não roda o comando que ela SABE que vai falhar.**
    ///
    /// Sem conta cadastrada, o `repos` sai com "nenhuma conta cadastrada — `schematize-git add`
    /// ou `detect`". Essa frase é de TERMINAL: ela manda digitar um comando para quem está numa
    /// janela com a aba do lado. Despejá-la numa caixa amarela é o software passando a conta
    /// para quem clicou (§37.48).
    #[test]
    fn sem_conta_a_aba_de_repos_nao_chama_o_gh() {
        let fonte = include_str!("main.rs");
        let producao = fonte.split("#[cfg(test)]").next().expect("há código antes dos testes");
        let i = producao.find("fn carregar_repos").expect("a função sumiu");
        let corpo = &producao[i..];
        let guarda = corpo.find("row_count() == 0").expect("a guarda de lista vazia sumiu");
        let chamada = corpo.find("\"repos\"").expect("a chamada ao `repos` sumiu");
        assert!(
            guarda < chamada,
            "a guarda tem de vir ANTES da chamada — depois, o comando já rodou e já falhou"
        );
        assert!(
            corpo[..chamada].contains("aba Contas"),
            "a mensagem tem de apontar a ABA, não um comando de terminal"
        );
    }

    /// **A casca não sabe NADA do domínio** (D4 do ADR-0016).
    ///
    /// Ela mora no mesmo repo do CLI (ADR-0020), e isso é conveniência de distribuição — não
    /// permissão para acoplar. Este teste lê o próprio fonte e reprova o uso do crate de
    /// domínio e qualquer `Command::new` direto.
    ///
    /// **`nucleo::` é permitido e o domínio não é**, e a distinção não é de nome: a janela usa
    /// `nucleo::procedencia` para dizer de qual COMMIT ela é, o que é plataforma. O que o
    /// ADR-0020 proíbe é a janela LER o domínio em vez de perguntar ao binário.
    #[test]
    fn a_casca_nao_sabe_nada_do_dominio() {
        let fonte = include_str!("main.rs");
        let producao = fonte.split("#[cfg(test)]").next().expect("há código antes dos testes");
        for proibido in
            ["git_casa::contas", "git_casa::repos", "git_casa::aplicar", "git_casa::historico"]
        {
            assert!(
                !producao.contains(proibido),
                "a janela usou `{proibido}`: ela fala com o BINÁRIO por `--json`, e depender do \
                 crate a faria embutir uma versão — o bug que fez a janela irmã abrir a versão \
                 antiga"
            );
        }
        // **O varredor ignora COMENTÁRIO**, e isso foi aprendido errando na janela do database:
        // a primeira versão reprovou por causa de um comentário que dizia `Command::new`.
        // Proibir a palavra proíbe explicar a regra. Comentário CITA; só a linha de código USA.
        let codigo: Vec<&str> =
            producao.lines().map(str::trim_start).filter(|l| !l.starts_with("//")).collect();
        for linha in &codigo {
            assert!(
                !linha.contains("Command::new"),
                "a janela rodou processo direto:\n  {linha}\nTudo passa pela casca \
                 (`cli`/`terminal`), que é onde o tratamento de erro vive"
            );
        }
        // Self-check: o varredor tem de VER o que procura quando ele está no código.
        assert!(
            codigo.iter().any(|l| l.contains("cli::rodar")),
            "a janela fala com o binário pela casca — se esta linha sumiu, o varredor está cego"
        );
    }

    /// **As ações que MUDAM a máquina vão para o terminal, e nenhuma escreve daqui** (D6).
    ///
    /// Aplicar identidade reescreve o `.git/config`; escrever alias mexe no `~/.ssh/config`.
    /// As duas podem pedir credencial, e uma janela Slint não tem como responder a um prompt —
    /// ela ficaria pendurada, e o erro morreria com ela.
    #[test]
    fn toda_escrita_passa_pelo_terminal() {
        let fonte = include_str!("main.rs");
        let producao = fonte.split("#[cfg(test)]").next().expect("há código antes dos testes");
        for (callback, sub) in
            [("on_usar_conta", "\"use\""), ("on_escrever_alias", "\"ssh-config\"")]
        {
            let i = producao.find(callback).unwrap_or_else(|| panic!("{callback} sumiu"));
            let corpo = &producao[i..producao.len().min(i + 400)];
            assert!(corpo.contains("no_terminal"), "{callback} não passa pelo terminal");
            assert!(corpo.contains(sub), "{callback} perdeu o subcomando {sub}");
        }
        // E a janela NÃO escreve arquivo nenhum por conta própria.
        let codigo: Vec<&str> =
            producao.lines().map(str::trim_start).filter(|l| !l.starts_with("//")).collect();
        for proibido in ["fs::write", "fs::create", "OpenOptions", "fs::remove"] {
            assert!(
                !codigo.iter().any(|l| l.contains(proibido)),
                "a janela escreveu no disco (`{proibido}`) — toda escrita é do binário, no terminal"
            );
        }
        // Self-check: o varredor acha o que procura quando ele está lá.
        assert!(codigo.iter().any(|l| l.contains("no_terminal(&w")), "o varredor está cego");
    }
}
