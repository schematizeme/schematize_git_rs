//! **market_parasita** — o gestor de atualizações viaja com qualquer programa da casa.
//!
//! **O quê:** ao iniciar, garante que `schematize-market` e `schematize-market-gui` existem em
//! `$HOME/.cargo/bin`; se faltar algum, baixa do release oficial e instala atomicamente.
//!
//! **Onde:** a primeira coisa do `main` de cada binário (CLI e janela), depois do parse de args.
//!
//! **Contrato (decisão do usuário, 2026-10-13):** nunca derruba o app (piso 10) — toda falha
//! vira UMA linha no stderr e o app segue; nada de `unwrap`/`expect` (piso 4). Opt-out:
//! `SCHEMATIZE_SEM_MARKET=1` ou `CI`; não roda quando o market chama o binário
//! (`SCHEMATIZE_DELEGADO_AO_MARKET=1`), nem em `--version`/`--help`/`desktop`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

/// Nome do binário de linha de comando do market.
pub const MARKET_CLI: &str = "schematize-market";
/// Nome da janela do market.
pub const MARKET_GUI: &str = "schematize-market-gui";
/// Idade a partir da qual um lock é considerado órfão.
const LOCK_VELHO: Duration = Duration::from_secs(10 * 60);
/// Comando de instalação manual citado na mensagem de falha.
const INSTALL_SH: &str =
    "curl -fsSL https://raw.githubusercontent.com/schematizeme/schematize-cli/main/install.sh | bash";

/// Desfecho de uma tentativa de garantir o market.
#[derive(Debug, PartialEq)]
pub enum Resultado {
    /// Os dois binários já existiam.
    NadaAFazer,
    /// Outro processo tem um lock recente e está cuidando.
    OutroCuidando,
    /// Instalou os binários listados.
    Instalou(Vec<&'static str>),
    /// Falhou; a causa vai na linha de stderr.
    Falhou(String),
}

/// Onde e para qual plataforma instalar (injetável nos testes).
pub struct Ambiente<'a> {
    /// Pasta canônica (`$HOME/.cargo/bin`).
    pub pasta: &'a Path,
    /// Arquivo de lock (`~/.schematize/run/market-bootstrap.lock`).
    pub lock: &'a Path,
    /// `linux`, `macos` ou `windows`.
    pub os: &'a str,
    /// `x86_64` ou `arm64`.
    pub arch: &'a str,
}

/// **O quê:** nome do arquivo no disco (`.exe` no Windows).
///
/// **Onde:** [`o_que_falta`] e [`instalar`].
pub fn nome_no_disco(nome: &str, os: &str) -> String {
    if os == "windows" {
        format!("{nome}.exe")
    } else {
        nome.to_string()
    }
}

/// **O quê:** quais dos dois binários do market não estão na pasta (2 `stat`, sem rede).
///
/// **Onde:** caminho rápido de [`instalar`]. Função pura de decisão.
pub fn o_que_falta(pasta: &Path) -> Vec<&'static str> {
    faltam_em(pasta, os_atual())
}

/// **O quê:** o mesmo que [`o_que_falta`] para um `os` explícito.
///
/// **Onde:** [`o_que_falta`] e [`instalar`].
fn faltam_em(pasta: &Path, os: &str) -> Vec<&'static str> {
    [MARKET_CLI, MARKET_GUI]
        .into_iter()
        .filter(|n| !pasta.join(nome_no_disco(n, os)).exists())
        .collect()
}

/// **O quê:** URL do asset do release `latest` para o binário e a plataforma, ou `None` se a
/// plataforma não tem asset publicado (publicados: linux x86_64, macos x86_64/arm64, windows
/// x86_64).
///
/// **Onde:** [`instalar`].
pub fn url_do_asset(nome: &str, os: &str, arch: &str) -> Option<String> {
    let publicado = matches!(
        (os, arch),
        ("linux", "x86_64") | ("macos", "x86_64") | ("macos", "arm64") | ("windows", "x86_64")
    );
    if !publicado {
        return None;
    }
    let ext = if os == "windows" { ".exe" } else { "" };
    Some(format!(
        "https://github.com/schematizeme/schematize_market_rs/releases/latest/download/{nome}-{os}-{arch}{ext}"
    ))
}

/// **O quê:** `os` no vocabulário do release (`linux|macos|windows`).
///
/// **Onde:** [`o_que_falta`] e [`garantir_no_inicio`].
fn os_atual() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macos",
        "windows" => "windows",
        _ => "linux",
    }
}

/// **O quê:** `arch` no vocabulário do release (`x86_64|arm64`; outras ficam como vierem).
///
/// **Onde:** [`garantir_no_inicio`].
fn arch_atual() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        outra => outra,
    }
}

/// **O quê:** baixa `url` para `dst` com `curl -fsSL --max-time 30`.
///
/// **Onde:** downloader real passado a [`instalar`] por [`garantir_no_inicio`].
pub fn baixar_com_curl(url: &str, dst: &Path) -> Result<(), String> {
    let st = Command::new("curl")
        .args(["-fsSL", "--max-time", "30", "-o"])
        .arg(dst)
        .arg(url)
        .stdin(Stdio::null())
        .status()
        .map_err(|e| format!("curl indisponível: {e}"))?;
    if st.success() {
        Ok(())
    } else {
        Err(format!("download falhou ({st})"))
    }
}

/// **O quê:** dá `chmod +x` (unix) e exige que `<p> --version` saia com 0.
///
/// **Onde:** validação real passada a [`instalar`], antes do `rename`.
pub fn validar_executavel(p: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(p, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod: {e}"))?;
    }
    let st = Command::new(p)
        .arg("--version")
        .env("SCHEMATIZE_SEM_MARKET", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("binário baixado não executa: {e}"))?;
    if st.success() {
        Ok(())
    } else {
        Err(format!("`--version` do binário baixado saiu com {st}"))
    }
}

/// **O quê:** toma o lock com `create_new`; `Ok(false)` se outro processo (lock < 10 min) cuida;
/// lock mais velho é órfão: remove e toma.
///
/// **Onde:** [`instalar`], só quando falta algum binário.
fn tomar_lock(lock: &Path) -> Result<bool, String> {
    if let Some(pai) = lock.parent() {
        fs::create_dir_all(pai).map_err(|e| format!("criar {}: {e}", pai.display()))?;
    }
    for _ in 0..2 {
        match fs::OpenOptions::new().write(true).create_new(true).open(lock) {
            Ok(mut f) => {
                // O pid é só informativo; falhar em escrevê-lo não invalida o lock.
                if let Err(e) = writeln!(f, "{}", std::process::id()) {
                    eprintln!("schematize: aviso: não gravei o pid no lock ({e})");
                }
                return Ok(true);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let idade = fs::metadata(lock)
                    .and_then(|m| m.modified())
                    .map(|t| SystemTime::now().duration_since(t).unwrap_or_default())
                    .map_err(|e| format!("ler o lock: {e}"))?;
                if idade < LOCK_VELHO {
                    return Ok(false);
                }
                fs::remove_file(lock).map_err(|e| format!("remover lock órfão: {e}"))?;
            }
            Err(e) => return Err(format!("criar o lock: {e}")),
        }
    }
    Ok(false)
}

/// **O quê:** baixa, valida e move para o lugar UM binário; qualquer falha remove o `.novo`.
///
/// **Onde:** [`instalar`], uma vez por binário que falta.
fn instalar_um(
    amb: &Ambiente,
    nome: &str,
    baixar: &dyn Fn(&str, &Path) -> Result<(), String>,
    validar: &dyn Fn(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let url = url_do_asset(nome, amb.os, amb.arch)
        .ok_or_else(|| format!("sem release de {nome} para {}-{}", amb.os, amb.arch))?;
    let dst = amb.pasta.join(nome_no_disco(nome, amb.os));
    let novo = PathBuf::from(format!("{}.novo", dst.display()));
    let passo = baixar(&url, &novo).and_then(|()| validar(&novo)).and_then(|()| {
        fs::rename(&novo, &dst).map_err(|e| format!("rename para {}: {e}", dst.display()))
    });
    if passo.is_err() && novo.exists() {
        if let Err(e) = fs::remove_file(&novo) {
            eprintln!("schematize: aviso: não removi {} ({e})", novo.display());
        }
    }
    passo.map_err(|e| format!("{nome}: {e}"))
}

/// **O quê:** o caminho de instalação — decide o que falta, toma o lock, instala cada faltante
/// e solta o lock. Downloader e validador são injetados (testes sem rede).
///
/// **Onde:** [`rodar`].
pub fn instalar(
    amb: &Ambiente,
    baixar: &dyn Fn(&str, &Path) -> Result<(), String>,
    validar: &dyn Fn(&Path) -> Result<(), String>,
) -> Resultado {
    let faltam = faltam_em(amb.pasta, amb.os);
    if faltam.is_empty() {
        return Resultado::NadaAFazer;
    }
    match tomar_lock(amb.lock) {
        Ok(true) => {}
        Ok(false) => return Resultado::OutroCuidando,
        Err(e) => return Resultado::Falhou(e),
    }
    let r = fs::create_dir_all(amb.pasta)
        .map_err(|e| format!("criar {}: {e}", amb.pasta.display()))
        .and_then(|()| faltam.iter().try_for_each(|n| instalar_um(amb, n, baixar, validar)));
    if let Err(e) = fs::remove_file(amb.lock) {
        eprintln!("schematize: aviso: não removi o lock ({e})");
    }
    match r {
        Ok(()) => Resultado::Instalou(faltam),
        Err(e) => Resultado::Falhou(e),
    }
}

/// **O quê:** decide se o bootstrap roda: não em `--version`/`-V`/`--help`/`-h`/`help`,
/// nem em `desktop`, nem com `SCHEMATIZE_SEM_MARKET=1`, `CI` ou
/// `SCHEMATIZE_DELEGADO_AO_MARKET=1`. `env(k)` diz se a variável vale (`SEM_MARKET` e
/// `DELEGADO` valem `1`; `CI` vale se definida).
///
/// **Onde:** [`rodar`]. Função pura.
pub fn deve_rodar(args: &[String], env: &dyn Fn(&str) -> bool) -> bool {
    if env("SCHEMATIZE_SEM_MARKET") || env("CI") || env("SCHEMATIZE_DELEGADO_AO_MARKET") {
        return false;
    }
    let pede_info = args.iter().any(|a| matches!(a.as_str(), "--version" | "-V" | "--help" | "-h"));
    let sub = args.first().map(String::as_str);
    !(pede_info || matches!(sub, Some("desktop") | Some("help")))
}

/// **O quê:** `deve_rodar` + `instalar`, com tudo injetável.
///
/// **Onde:** [`garantir_no_inicio`] e os testes. `None` = não rodou.
pub fn rodar(
    args: &[String],
    env: &dyn Fn(&str) -> bool,
    amb: &Ambiente,
    baixar: &dyn Fn(&str, &Path) -> Result<(), String>,
    validar: &dyn Fn(&Path) -> Result<(), String>,
) -> Option<Resultado> {
    deve_rodar(args, env).then(|| instalar(amb, baixar, validar))
}

/// **O quê:** a linha de stderr de cada desfecho (`None` = silêncio).
///
/// **Onde:** [`garantir_no_inicio`].
fn mensagem(r: &Resultado, pasta: &Path) -> Option<String> {
    match r {
        Resultado::NadaAFazer | Resultado::OutroCuidando => None,
        Resultado::Instalou(n) => Some(format!(
            "schematize: instalei o gestor de atualizações ({}) em {}",
            n.join(", "),
            pasta.display()
        )),
        Resultado::Falhou(c) => Some(format!(
            "schematize: não consegui instalar o schematize-market ({c}) — o app segue normal; rode: {INSTALL_SH}"
        )),
    }
}

/// **O quê:** o ponto de entrada: garante o market na pasta canônica, imprime a linha de
/// stderr e SEMPRE retorna (o app nunca cai por causa disto).
///
/// **Onde:** início do `main` de cada binário, depois do parse de args. `args` exclui `argv[0]`.
/// Efeitos: rede (`curl`), arquivos em `~/.cargo/bin` e `~/.schematize/run`. Inerte em `cfg(test)`.
pub fn garantir_no_inicio(args: &[String]) {
    if cfg!(test) {
        return;
    }
    let env = |k: &str| match std::env::var_os(k) {
        Some(v) => k == "CI" || v == "1",
        None => false,
    };
    if !deve_rodar(args, &env) {
        return;
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .filter(|h| !h.is_empty())
        .map(PathBuf::from);
    let Some(home) = home else {
        let r = Resultado::Falhou("HOME indefinido".into());
        if let Some(m) = mensagem(&r, Path::new("?")) {
            eprintln!("{m}");
        }
        return;
    };
    let pasta = home.join(".cargo").join("bin");
    let lock = home.join(".schematize").join("run").join("market-bootstrap.lock");
    let amb = Ambiente { pasta: &pasta, lock: &lock, os: os_atual(), arch: arch_atual() };
    let r = instalar(&amb, &baixar_com_curl, &validar_executavel);
    if let Some(m) = mensagem(&r, &pasta) {
        eprintln!("{m}");
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::cell::Cell;
    use std::fs;
    use std::path::PathBuf;

    /// **O quê:** cria um diretório temporário único dentro do alvo de build (nunca no HOME).
    ///
    /// **Onde:** todos os testes deste módulo.
    fn tmp(nome: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("market_parasita_{}_{}", nome, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// **O quê:** baixador falso que escreve um script executável que sai 0 (ou lixo).
    ///
    /// **Onde:** testes de `instalar`.
    fn baixa_ok(_url: &str, dst: &Path) -> Result<(), String> {
        fs::write(dst, "#!/bin/sh\nexit 0\n").map_err(|e| e.to_string())
    }

    /// **O quê:** baixador falso que entrega um arquivo que não executa.
    ///
    /// **Onde:** teste do binário quebrado.
    fn baixa_lixo(_url: &str, dst: &Path) -> Result<(), String> {
        fs::write(dst, "isto nao e um executavel").map_err(|e| e.to_string())
    }

    #[test]
    fn falta_os_dois_um_ou_nenhum() {
        let d = tmp("falta");
        assert_eq!(o_que_falta(&d), vec![MARKET_CLI, MARKET_GUI]);
        fs::write(d.join(nome_no_disco(MARKET_CLI, os_atual())), "x").unwrap();
        assert_eq!(o_que_falta(&d), vec![MARKET_GUI]);
        fs::write(d.join(nome_no_disco(MARKET_GUI, os_atual())), "x").unwrap();
        assert!(o_que_falta(&d).is_empty());
    }

    #[test]
    fn url_dos_quatro_alvos_e_um_sem_asset() {
        let base = "https://github.com/schematizeme/schematize_market_rs/releases/latest/download";
        assert_eq!(
            url_do_asset(MARKET_CLI, "linux", "x86_64").unwrap(),
            format!("{base}/schematize-market-linux-x86_64")
        );
        assert_eq!(
            url_do_asset(MARKET_GUI, "macos", "arm64").unwrap(),
            format!("{base}/schematize-market-gui-macos-arm64")
        );
        assert_eq!(
            url_do_asset(MARKET_CLI, "macos", "x86_64").unwrap(),
            format!("{base}/schematize-market-macos-x86_64")
        );
        assert_eq!(
            url_do_asset(MARKET_GUI, "windows", "x86_64").unwrap(),
            format!("{base}/schematize-market-gui-windows-x86_64.exe")
        );
        assert!(url_do_asset(MARKET_CLI, "linux", "arm64").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn instala_com_rename_e_sem_novo_sobrando() {
        let d = tmp("ok");
        let (pasta, lock) = (d.join("bin"), d.join("run/market-bootstrap.lock"));
        let amb = Ambiente { pasta: &pasta, lock: &lock, os: "linux", arch: "x86_64" };
        let r = instalar(&amb, &baixa_ok, &validar_executavel);
        assert_eq!(r, Resultado::Instalou(vec![MARKET_CLI, MARKET_GUI]));
        assert!(o_que_falta(&pasta).is_empty());
        let sobras: Vec<_> = fs::read_dir(&pasta)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".novo"))
            .collect();
        assert!(sobras.is_empty());
        assert!(!lock.exists(), "o lock tem de ser removido ao fim");
    }

    #[cfg(unix)]
    #[test]
    fn binario_que_nao_executa_remove_o_novo_e_nao_toca_o_destino() {
        let d = tmp("lixo");
        let (pasta, lock) = (d.join("bin"), d.join("run/l.lock"));
        let amb = Ambiente { pasta: &pasta, lock: &lock, os: "linux", arch: "x86_64" };
        let r = instalar(&amb, &baixa_lixo, &validar_executavel);
        assert!(matches!(r, Resultado::Falhou(_)), "{r:?}");
        assert_eq!(o_que_falta(&pasta), vec![MARKET_CLI, MARKET_GUI]);
        let restos = fs::read_dir(&pasta).map(|i| i.count()).unwrap_or(0);
        assert_eq!(restos, 0, "nem .novo nem destino");
        assert!(!lock.exists());
    }

    #[test]
    fn lock_recente_nao_faz_nada() {
        let d = tmp("lockrec");
        let (pasta, lock) = (d.join("bin"), d.join("run/l.lock"));
        fs::create_dir_all(lock.parent().unwrap()).unwrap();
        fs::write(&lock, "outro").unwrap();
        let chamadas = Cell::new(0);
        let baixa = |_: &str, _: &Path| {
            chamadas.set(chamadas.get() + 1);
            Ok(())
        };
        let amb = Ambiente { pasta: &pasta, lock: &lock, os: "linux", arch: "x86_64" };
        assert_eq!(instalar(&amb, &baixa, &validar_executavel), Resultado::OutroCuidando);
        assert_eq!(chamadas.get(), 0);
        assert!(lock.exists(), "lock alheio não pode ser apagado");
    }

    #[cfg(unix)]
    #[test]
    fn lock_velho_e_orfao_e_o_processo_segue() {
        let d = tmp("lockvelho");
        let (pasta, lock) = (d.join("bin"), d.join("run/l.lock"));
        fs::create_dir_all(lock.parent().unwrap()).unwrap();
        let f = fs::File::create(&lock).unwrap();
        f.set_modified(std::time::SystemTime::now() - Duration::from_secs(11 * 60)).unwrap();
        let amb = Ambiente { pasta: &pasta, lock: &lock, os: "linux", arch: "x86_64" };
        let r = instalar(&amb, &baixa_ok, &validar_executavel);
        assert_eq!(r, Resultado::Instalou(vec![MARKET_CLI, MARKET_GUI]));
    }

    #[test]
    fn plataforma_sem_asset_nao_baixa_e_diz_o_motivo() {
        let d = tmp("semasset");
        let (pasta, lock) = (d.join("bin"), d.join("l.lock"));
        let chamadas = Cell::new(0);
        let baixa = |_: &str, _: &Path| {
            chamadas.set(chamadas.get() + 1);
            Ok(())
        };
        let amb = Ambiente { pasta: &pasta, lock: &lock, os: "linux", arch: "arm64" };
        let r = instalar(&amb, &baixa, &validar_executavel);
        assert!(matches!(&r, Resultado::Falhou(m) if m.contains("linux-arm64")), "{r:?}");
        assert_eq!(chamadas.get(), 0);
    }

    #[test]
    fn opt_out_nao_chama_o_downloader() {
        let d = tmp("optout");
        let (pasta, lock) = (d.join("bin"), d.join("l.lock"));
        let amb = Ambiente { pasta: &pasta, lock: &lock, os: "linux", arch: "x86_64" };
        let chamadas = Cell::new(0);
        let baixa = |_: &str, _: &Path| {
            chamadas.set(chamadas.get() + 1);
            Ok(())
        };
        for var in ["SCHEMATIZE_SEM_MARKET", "CI", "SCHEMATIZE_DELEGADO_AO_MARKET"] {
            let env = |k: &str| k == var;
            assert!(rodar(&[], &env, &amb, &baixa, &validar_executavel).is_none(), "{var}");
        }
        assert_eq!(chamadas.get(), 0);
        // Sem o opt-out, o mesmo cenário CHAMA o downloader (prova que o teste não é cego).
        let sem_env = |_: &str| false;
        assert!(rodar(&[], &sem_env, &amb, &baixa, &validar_executavel).is_some());
        assert!(chamadas.get() > 0);
    }

    #[test]
    fn nao_roda_em_version_help_e_desktop() {
        let sem_env = |_: &str| false;
        assert!(deve_rodar(&[], &sem_env));
        assert!(deve_rodar(&["sql".to_string()], &sem_env));
        for a in ["--version", "-V", "--help", "-h"] {
            assert!(!deve_rodar(&[a.to_string()], &sem_env), "{a}");
        }
        let desk = ["desktop".to_string(), "--install".to_string()];
        assert!(!deve_rodar(&desk, &sem_env));
    }
}
