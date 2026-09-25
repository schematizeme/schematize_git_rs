//! O `--json` é CONTRATO com outro repositório — e este arquivo é quem o trava.
//!
//! **Onde:** `cargo test`, e o CI.
//!
//! **Por que de integração e não unitário:** o contrato é o que SAI do binário. Um teste que
//! chamasse a função de formatação provaria a função; este roda o comando e lê o `stdout`, que é
//! o que a janela vê de verdade.
//!
//! ## Todo teste daqui roda num HOME próprio
//!
//! O app lê o cadastro de `~/.claude/schematize/`. Um teste que usasse o HOME real leria as
//! contas de quem está rodando a suíte — e, pior, o `detect --add` as ESCREVERIA. Cada teste
//! monta um HOME temporário e o descarta.

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_schematize-git");

/// Um HOME temporário, isolado por teste.
///
/// **Onde:** todos os testes daqui. O nome inclui o PID e o nome do teste — dois testes em
/// paralelo não podem disputar o mesmo cadastro.
fn home(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("gitrs-{}-{nome}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join(".claude/schematize")).expect("criar o HOME de teste");
    d
}

/// Roda o binário com um HOME dado; devolve `(stdout, stderr, sucesso)`.
fn rodar_em(h: &Path, cwd: &Path, args: &[&str]) -> (String, String, bool) {
    let o = Command::new(BIN)
        .args(args)
        .current_dir(cwd)
        .env("HOME", h)
        // As SEIS variáveis de idioma: o documento não pode mudar com o ambiente, e deixar uma
        // de fora faria o teste passar por acidente numa máquina já configurada.
        .env_remove("LANGUAGE")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LC_CTYPE")
        .env_remove("SCHEMATIZE_LANG")
        .output()
        .expect("rodar o binário");
    (
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
        o.status.success(),
    )
}

fn rodar(h: &Path, args: &[&str]) -> (String, String, bool) {
    rodar_em(h, h, args)
}

/// Cadastra uma conta de teste.
fn cadastrar(h: &Path, rotulo: &str, chave: Option<&str>) {
    let mut args = vec!["add", rotulo, "--usuario", "luna", "--email", "luna@exemplo.com"];
    if let Some(k) = chave {
        args.push("--chave");
        args.push(k);
    }
    let (_, err, ok) = rodar(h, &args);
    assert!(ok, "cadastrar falhou: {err}");
}

/// **O CONTRATO do `accounts --json`.** Renomear um campo faz este teste mostrar a diferença.
#[test]
fn o_shape_das_contas_e_contrato() {
    let h = home("contas");
    cadastrar(&h, "pessoal", Some("id_ed25519_pessoal"));
    let (out, err, ok) = rodar(&h, &["accounts", "--json"]);
    assert!(ok, "falhou: {err}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("tem de ser JSON válido");

    let topo: Vec<&str> = v.as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
    assert_eq!(topo, ["contas"]);

    let c = &v["contas"][0];
    let chaves: Vec<&str> = c.as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
    assert_eq!(chaves, ["alias_ok", "auth", "email", "rotulo", "servico", "usuario"]);

    // O `auth` é objeto com slug FECHADO — a janela ramifica por ele.
    let a: Vec<&str> = c["auth"].as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
    assert_eq!(a, ["chave", "tipo"]);
    assert_eq!(c["auth"]["tipo"], "ssh");

    // Conta `gh` tem `chave: null`, e `null` NÃO é `""`.
    cadastrar(&h, "trabalho", None);
    let (out, _, _) = rodar(&h, &["accounts", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("JSON válido");
    let gh = v["contas"]
        .as_array()
        .expect("lista")
        .iter()
        .find(|c| c["rotulo"] == "trabalho")
        .expect("a conta existe");
    assert_eq!(gh["auth"]["tipo"], "gh");
    assert!(gh["auth"]["chave"].is_null(), "sem chave é null, não string vazia");
    let _ = std::fs::remove_dir_all(&h);
}

/// **REGRESSÃO.** A primeira versão do `accounts --json` montava o objeto e tirava a chave
/// final com `trim_end_matches('}')` para acrescentar o `alias_ok`.
///
/// `trim_end_matches` remove **todas** as ocorrências finais — e a conta termina em `}}`, porque
/// o último campo é o objeto do `auth`. O documento saía **inválido**, e o erro só aparecia num
/// parser estrito: `Expecting ',' delimiter`. Um teste que casasse substring teria aprovado.
#[test]
fn o_documento_de_contas_e_json_valido_com_auth_ssh() {
    let h = home("regressao");
    // A conta SSH é o caso: o `auth` dela é um objeto, então a conta termina em `}}`.
    cadastrar(&h, "pessoal", Some("id_ed25519"));
    let (out, _, ok) = rodar(&h, &["accounts", "--json"]);
    assert!(ok);
    let v: serde_json::Value =
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("documento inválido: {e}\n{out}"));
    assert_eq!(v["contas"][0]["alias_ok"], false, "o campo que a cirurgia de string quebrava");
    let _ = std::fs::remove_dir_all(&h);
}

/// **O CONTRATO do `status --json`.**
#[test]
fn o_shape_do_status_e_contrato() {
    let h = home("status");
    let proj = h.join("projetos/um");
    std::fs::create_dir_all(&proj).expect("criar projeto");
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&proj)
        .status()
        .expect("git")
        .success());
    // O repo precisa ter ALGO por sair, senão o domínio o filtra — e o filtro está certo: um
    // repositório limpo, sem remoto e sem commit, não tem nada "que ainda não saiu da máquina".
    // Um arquivo não rastreado o torna sujo, e é o caso que também prova `remoto: null`.
    std::fs::write(proj.join("arquivo.txt"), "conteudo").expect("sujar o repo");
    let (out, err, ok) =
        rodar(&h, &["status", "--json", "--dir", h.join("projetos").to_str().unwrap()]);
    assert!(ok, "falhou: {err}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("JSON válido");
    assert_eq!(
        v.as_object().expect("objeto").keys().map(|s| s.as_str()).collect::<Vec<_>>(),
        ["projetos"]
    );
    let p = &v["projetos"][0];
    let chaves: Vec<&str> = p.as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
    assert_eq!(chaves, ["conta", "email", "nao_enviados", "nome", "raiz", "remoto", "sujo"]);
    // Repo sem remoto: `null`, não `""`. A janela desenha os dois casos diferente.
    assert!(p["remoto"].is_null(), "sem remoto é null: {p}");
    let _ = std::fs::remove_dir_all(&h);
}

/// **O CONTRATO do `detect --json`, e a distinção entre TIPO de fonte e QUAL arquivo.**
#[test]
fn o_shape_do_detect_e_contrato() {
    let h = home("detect");
    let (out, err, ok) = rodar(&h, &["detect", "--json"]);
    assert!(ok, "falhou: {err}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("JSON válido");
    assert_eq!(
        v.as_object().expect("objeto").keys().map(|s| s.as_str()).collect::<Vec<_>>(),
        ["sugestoes"]
    );
    // A lista pode vir vazia (nada configurado no HOME de teste) — o que se trava é o shape
    // quando há item, e o teste não pode exigir que a máquina do CI tenha `gh` logado.
    if let Some(s) = v["sugestoes"].as_array().and_then(|a| a.first()) {
        let chaves: Vec<&str> = s.as_object().expect("objeto").keys().map(|s| s.as_str()).collect();
        assert_eq!(chaves, ["conta", "ja_cadastrada", "origem", "origem_onde"]);
        let slug = s["origem"].as_str().expect("slug");
        assert!(
            ["gh", "git-global", "ssh-config", "repo"].contains(&slug),
            "`origem` é conjunto FECHADO, e veio `{slug}` — a janela ramifica por ele"
        );
    }
    let _ = std::fs::remove_dir_all(&h);
}

/// **`detect --json` NÃO grava, nem com `--add`.**
///
/// Um comando de leitura que escreve é uma armadilha: quem pede um documento de máquina está
/// perguntando o estado, e a janela chama isso a cada atualização de tela.
#[test]
fn detect_json_nunca_grava_nem_com_add() {
    let h = home("detect-nao-grava");
    let cadastro = h.join(".claude/schematize");
    let antes = std::fs::read_dir(&cadastro).expect("dir").flatten().count();
    let (_, _, ok) = rodar(&h, &["detect", "--json", "--add"]);
    assert!(ok);
    let depois = std::fs::read_dir(&cadastro).expect("dir").flatten().count();
    assert_eq!(antes, depois, "`detect --json --add` não pode criar arquivo nenhum");
    let _ = std::fs::remove_dir_all(&h);
}

/// **O documento é byte-a-byte IGUAL em qualquer idioma.**
#[test]
fn o_json_e_o_mesmo_em_qualquer_idioma() {
    let h = home("idioma");
    cadastrar(&h, "pessoal", Some("id_ed25519"));
    let mut saidas = Vec::new();
    for lang in ["en_US.UTF-8", "pt_BR.UTF-8", "C", "ja_JP.UTF-8"] {
        let o = Command::new(BIN)
            .args(["accounts", "--json"])
            .current_dir(&h)
            .env("HOME", &h)
            .env_remove("LANGUAGE")
            .env_remove("LC_ALL")
            .env_remove("LC_MESSAGES")
            .env_remove("LC_CTYPE")
            .env_remove("SCHEMATIZE_LANG")
            .env("LANG", lang)
            .output()
            .expect("rodar");
        assert!(o.status.success(), "falhou em {lang}");
        saidas.push(String::from_utf8_lossy(&o.stdout).to_string());
    }
    for s in saidas.iter().skip(1) {
        assert_eq!(&saidas[0], s, "o documento mudou com o idioma");
    }
    let _ = std::fs::remove_dir_all(&h);
}

/// **Texto hostil vem de FORA** — rótulo de conta e e-mail são digitados pela pessoa, e o
/// assunto de um commit vem do `git log`. Aspas e barra têm de sair escapadas.
#[test]
fn texto_hostil_no_cadastro_nao_quebra_o_documento() {
    let h = home("hostil");
    let (_, err, ok) = rodar(
        &h,
        &[
            "add",
            "tem\"aspas\\e-barra",
            "--usuario",
            "u\"x",
            "--email",
            "a\\b@exemplo.com",
            "--chave",
            "k\"1",
        ],
    );
    assert!(ok, "cadastrar falhou: {err}");
    let (out, _, ok) = rodar(&h, &["accounts", "--json"]);
    assert!(ok);
    let v: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("documento inválido com texto hostil: {e}\n{out}"));
    let c = &v["contas"][0];
    assert_eq!(c["rotulo"], "tem\"aspas\\e-barra", "a aspa tem de voltar como aspa");
    assert_eq!(c["usuario"], "u\"x");
    assert_eq!(c["auth"]["chave"], "k\"1");
    let _ = std::fs::remove_dir_all(&h);
}

/// Conta que não existe: erro nomeando o rótulo, e código de saída 1.
#[test]
fn conta_inexistente_da_erro_nomeando_o_rotulo() {
    let h = home("inexistente");
    let (_, err, ok) = rodar(&h, &["use", "nao-existe"]);
    assert!(!ok, "tinha de sair 1");
    assert!(err.contains("nao-existe"), "o erro tem de nomear o que faltou: {err}");
    assert!(!err.contains("panicked"), "panicou: {err}");
    let _ = std::fs::remove_dir_all(&h);
}

/// Sem conta nenhuma, `repos` explica o que fazer em vez de listar vazio.
#[test]
fn sem_conta_o_erro_e_acionavel() {
    let h = home("sem-conta");
    let (_, err, ok) = rodar(&h, &["repos"]);
    assert!(!ok);
    assert!(err.contains("add") && err.contains("detect"), "§37.48 — diga o que fazer: {err}");
    let _ = std::fs::remove_dir_all(&h);
}

/// **A versão diz de qual COMMIT o binário é.**
#[test]
fn a_versao_traz_a_procedencia() {
    let h = home("versao");
    let (out, _, ok) = rodar(&h, &["--version"]);
    assert!(ok);
    assert!(out.contains('('), "sem procedência: {out}");
    let _ = std::fs::remove_dir_all(&h);
}
