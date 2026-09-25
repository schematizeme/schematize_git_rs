//! A borda de LINHA DE COMANDO — o despacho e o que cada subcomando imprime.
//!
//! **Onde:** `main`. O domínio está nos módulos da raiz do lib; aqui é só entrada e saída.
//!
//! ## A regra que este arquivo segue, e por que ela importa aqui
//!
//! Todo subcomando com `--json` tem **dois caminhos e uma só leitura**: lê o domínio uma vez e
//! escolhe entre imprimir para gente ou para máquina. Ler duas vezes — uma por caminho — abriria
//! a porta para os dois discordarem, que é exatamente o defeito que este ecossistema já pagou
//! (a janela dizendo "não instalado" sobre o que o terminal listava como instalado).

pub mod args;
pub mod saidajson;

use args::Cmd;
use git_casa::contas::{Auth, Conta};
use git_casa::nucleo::config;
use git_casa::{aplicar, contas, deteccao, historico, repos};

/// **O quê:** despacha o subcomando.
///
/// **Onde:** `main`. Devolve `Result` em vez de imprimir e sair aqui dentro, porque o código de
/// saída é parte do contrato de quem encadeia este comando num script.
pub fn executar(cmd: Cmd) -> Result<(), String> {
    match cmd {
        Cmd::Accounts { json } => listar_contas(json),
        Cmd::Add { rotulo, usuario, email, chave, servico } => {
            adicionar(rotulo, usuario, email, chave, servico)
        }
        Cmd::Detect { add, json } => detectar(add, json),
        Cmd::Remove { rotulo } => {
            if contas::remover(&rotulo)? {
                println!("conta '{rotulo}' removida.");
            } else {
                println!("não havia conta '{rotulo}'.");
            }
            Ok(())
        }
        Cmd::Use { rotulo, remoto } => usar(rotulo, remoto),
        Cmd::SshConfig { rotulo } => ssh_config(rotulo),
        Cmd::Repos { rotulo, limite, json } => listar_repos(rotulo, limite, json),
        Cmd::Status { dirs, json } => status(dirs, json),
        Cmd::Log { limite, json } => log(limite, json),
    }
}

fn listar_contas(json: bool) -> Result<(), String> {
    // O `alias_ok` é calculado AQUI e viaja no documento: a janela precisa dele para desenhar o
    // estado, e recalculá-lo do lado dela exigiria reimplementar a leitura do `~/.ssh/config`.
    let v: Vec<(Conta, bool)> =
        contas::listar().into_iter().map(|c| (c.clone(), aplicar::alias_configurado(&c))).collect();
    if json {
        println!("{}", saidajson::contas(&v));
        return Ok(());
    }
    if v.is_empty() {
        println!("nenhuma conta cadastrada.");
        println!(
            "  schematize-git add <rótulo> --usuario <login> --email <e-mail> \
             [--chave <arquivo em ~/.ssh>]"
        );
        println!("  ou `schematize-git detect` para ver o que já existe nesta máquina.");
        return Ok(());
    }
    for (c, alias_ok) in v {
        let auth = match &c.auth {
            Auth::Ssh { chave } => {
                let marca = if alias_ok { "alias ok" } else { "\x1b[33mFALTA alias\x1b[0m" };
                format!("ssh:{chave} ({marca})")
            }
            Auth::Gh => "gh".to_string(),
        };
        println!(
            "  \x1b[1m{:<12}\x1b[0m {} <{}>  {}  {auth}",
            c.rotulo, c.usuario, c.email, c.servico
        );
    }
    Ok(())
}

fn adicionar(
    rotulo: String,
    usuario: String,
    email: String,
    chave: Option<String>,
    servico: Option<String>,
) -> Result<(), String> {
    let c = Conta {
        rotulo: rotulo.clone(),
        usuario,
        email,
        servico: servico.unwrap_or_else(|| "github.com".into()),
        auth: match chave {
            Some(k) => Auth::Ssh { chave: k },
            None => Auth::Gh,
        },
    };
    contas::adicionar(c.clone())?;
    println!("conta '{rotulo}' cadastrada.");
    // O aviso é ACIONÁVEL (§37.48): diz o comando, não que a pessoa esqueceu algo.
    if matches!(c.auth, Auth::Ssh { .. }) && !aplicar::alias_configurado(&c) {
        println!("falta o alias SSH — rode: schematize-git ssh-config {rotulo}");
    }
    Ok(())
}

/// `detect` — e a linha que separa SUGERIR de GRAVAR.
///
/// **Sem `--add`, nada é escrito.** Um detector que cadastrasse o que achou transformaria um
/// palpite (um e-mail encontrado num repo) em fato registrado, e desfazer isso depois é mais
/// caro que cadastrar à mão.
fn detectar(add: bool, json: bool) -> Result<(), String> {
    let repositorios: Vec<std::path::PathBuf> = config::recent_projects()
        .into_iter()
        .map(std::path::PathBuf::from)
        .filter(|p| p.join(".git").exists())
        .collect();
    let sugestoes = deteccao::detectar(&repositorios);

    if json {
        // O `--json` NÃO grava, nem com `--add`: quem lê um documento de máquina está
        // perguntando o estado, e um comando de leitura que escreve é uma armadilha.
        println!("{}", saidajson::deteccao(&sugestoes));
        return Ok(());
    }
    if sugestoes.is_empty() {
        println!(
            "nenhuma conta detectada (sem `gh` logado, sem git config --global \
             e sem ~/.ssh/config)."
        );
        return Ok(());
    }
    println!("{} conta(s) detectada(s):\n", sugestoes.len());
    for s in &sugestoes {
        let auth = match &s.conta.auth {
            Auth::Ssh { chave } => format!("ssh:{chave}"),
            Auth::Gh => "gh".into(),
        };
        let marca = if s.ja_cadastrada { " [já cadastrada]" } else { "" };
        println!(
            "  {} · {}@{} · {} · auth {} · via {}{}",
            s.conta.rotulo,
            s.conta.usuario,
            s.conta.servico,
            if s.conta.email.is_empty() { "(sem e-mail)" } else { &s.conta.email },
            auth,
            s.origem.descricao(),
            marca
        );
    }
    if !add {
        println!("\nNada foi gravado. Use `schematize-git detect --add` pra cadastrar as novas.");
        return Ok(());
    }
    let mut n = 0;
    for s in sugestoes {
        if s.ja_cadastrada {
            continue;
        }
        // Conta sem e-mail é conta que faria o commit sair com identidade vazia. Pular é o
        // certo, e dizer POR QUE é o que faz a pessoa saber o que fazer.
        if s.conta.email.is_empty() {
            println!(
                "  ! {} pulada: sem e-mail. Cadastre à mão com `schematize-git add --email`.",
                s.conta.rotulo
            );
            continue;
        }
        match contas::adicionar(s.conta.clone()) {
            Ok(()) => {
                println!("  + {} cadastrada.", s.conta.rotulo);
                n += 1;
            }
            // Piso 4: a falha de UMA conta não interrompe as outras, e não some.
            Err(e) => println!("  ! {} falhou: {e}", s.conta.rotulo),
        }
    }
    println!("\n{n} conta(s) nova(s) cadastrada(s).");
    Ok(())
}

fn ssh_config(rotulo: String) -> Result<(), String> {
    let c = contas::por_rotulo(&rotulo).ok_or_else(|| format!("conta '{rotulo}' não existe"))?;
    if aplicar::escreve_alias(&c)? {
        println!("alias adicionado ao ~/.ssh/config:\n{}", c.bloco_ssh_config());
    } else {
        println!("nada a fazer (conta `gh` ou alias já configurado).");
    }
    Ok(())
}

fn usar(rotulo: String, remoto: Option<String>) -> Result<(), String> {
    let c = contas::por_rotulo(&rotulo).ok_or_else(|| format!("conta '{rotulo}' não existe"))?;
    let raiz = std::env::current_dir().map_err(|e| e.to_string())?;
    let feitos = aplicar::aplicar(&raiz, &c, remoto.as_deref().unwrap_or("origin"))?;
    println!("repositório {} agora usa a conta '{rotulo}':", raiz.display());
    for f in feitos {
        println!("  {f}");
    }
    Ok(())
}

fn listar_repos(rotulo: Option<String>, limite: usize, json: bool) -> Result<(), String> {
    let cs = match rotulo {
        Some(r) => vec![contas::por_rotulo(&r).ok_or_else(|| format!("conta '{r}' não existe"))?],
        None => contas::listar(),
    };
    if cs.is_empty() {
        return Err("nenhuma conta cadastrada — `schematize-git add` ou `detect`".into());
    }
    // O erro é POR CONTA: uma conta sem `gh` logado não pode apagar da tela os repositórios das
    // outras. Por isso o resultado de cada uma viaja como `Result`, e não um `?` aqui.
    let resultados: Vec<(String, Result<Vec<repos::Remoto>, String>)> =
        cs.iter().map(|c| (c.rotulo.clone(), repos::listar(c, limite))).collect();

    if json {
        println!("{}", saidajson::repositorios(&resultados));
        return Ok(());
    }
    for (rotulo, r) in &resultados {
        let usuario = cs.iter().find(|c| &c.rotulo == rotulo).map(|c| c.usuario.clone());
        println!("\n\x1b[1m{}\x1b[0m ({}):", rotulo, usuario.unwrap_or_default());
        match r {
            Ok(rs) => {
                for x in rs {
                    let vis = if x.privado { "privado" } else { "público" };
                    println!("  {:<40} {:<8} {}  {}", x.caminho, vis, x.atualizado, x.descricao);
                }
            }
            Err(e) => println!("  \x1b[33m{e}\x1b[0m"),
        }
    }
    Ok(())
}

/// `status` — o que ainda NÃO saiu da máquina, que é a pergunta que o git não responde sozinho.
fn status(dirs: Vec<String>, json: bool) -> Result<(), String> {
    // `--dir` explícito ganha da config: quem passou o diretório quer AQUELE.
    let alvos = if dirs.is_empty() { config::dev_dirs() } else { dirs };
    let estados = repos::estado_dos_projetos(&alvos);

    if json {
        println!("{}", saidajson::estado(&estados));
        return Ok(());
    }
    if estados.is_empty() {
        println!("nenhum projeto git nos diretórios de dev.");
        println!("  cadastre um com a janela do hub, ou passe `--dir <caminho>`.");
        return Ok(());
    }
    println!("{:<26} {:<12} {:>10}  ESTADO", "PROJETO", "CONTA", "NÃO ENVIADOS");
    let mut risco = 0usize;
    for e in &estados {
        let conta = e.conta.clone().unwrap_or_else(|| format!("? {}", e.email));
        let sujo = if e.sujo { "sujo" } else { "" };
        let cor = if e.nao_enviados > 0 { "\x1b[33m" } else { "" };
        println!("{:<26} {:<12} {cor}{:>10}\x1b[0m  {sujo}", e.nome, conta, e.nao_enviados);
        if e.nao_enviados > 0 {
            risco += 1;
        }
    }
    if risco > 0 {
        println!("\n\x1b[33m{risco} projeto(s) com commit que só existe nesta máquina.\x1b[0m");
    }
    Ok(())
}

/// `log` — os commits do projeto atual, marcando os que já foram enviados.
fn log(limite: usize, json: bool) -> Result<(), String> {
    let raiz = std::env::current_dir().map_err(|e| e.to_string())?;
    let up = historico::upstream(&raiz);
    let cs = historico::commits(&raiz, limite);

    if json {
        println!("{}", saidajson::log(up.as_ref(), &cs));
        return Ok(());
    }
    if let Some(u) = &up {
        let remoto = u.remote.clone().unwrap_or_else(|| "(sem upstream)".into());
        println!("{} -> {}  (+{} / -{})", u.branch, remoto, u.ahead, u.behind);
    }
    for c in &cs {
        let marca = if c.pushed { "\x1b[32m✓\x1b[0m" } else { "\x1b[33m↑\x1b[0m" };
        println!("{marca} {} {} {:<14} {}", c.short, c.date, elide(&c.author, 14), c.subject);
    }
    println!("\n\x1b[32m✓\x1b[0m já enviado   \x1b[33m↑\x1b[0m só nesta máquina");
    Ok(())
}

/// **O quê:** corta um texto no comprimento dado, contando CARACTERES e não bytes.
///
/// **Onde:** a coluna de autor do `log`.
///
/// **Por caractere:** um nome com acento tem mais bytes que letras, e cortar por byte parte um
/// caractere ao meio — o terminal mostra lixo, e em Rust um slice nesse ponto PANICA.
fn elide(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    s.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Corte por caractere, não por byte — nome com acento não vira lixo nem panica.
    #[test]
    fn elide_conta_caractere_e_nao_byte() {
        assert_eq!(elide("curto", 14), "curto");
        assert_eq!(elide("0123456789abcdef", 10), "012345678…");
        // 14 caracteres, 20 bytes em UTF-8: cortar por byte partiria um `ç` ao meio.
        let acentuado = "ãçõéíúàêôû-abc";
        assert_eq!(acentuado.chars().count(), 14);
        assert!(acentuado.len() > 14, "o caso só prova algo se bytes != caracteres");
        assert_eq!(elide(acentuado, 14), acentuado, "cabe: sai inteiro");
        assert_eq!(elide(acentuado, 5).chars().count(), 5);
    }
}
