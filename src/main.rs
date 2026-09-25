//! **schematize-git** — quem faz push, de onde, e com qual identidade.
//!
//! **Onde:** o ícone do app, a janela (por `--json`), e a mão de quem digita.

mod cli;

use clap::Parser;
use cli::args::Cli;

/// **O quê:** devolve o `SIGPIPE` ao padrão do Unix.
///
/// **Onde:** a primeira linha de `main`.
///
/// **Por quê:** o Rust ignora `SIGPIPE`, então `schematize-git log | head` faz o processo
/// PANICAR ao escrever num cano fechado, em vez de terminar em silêncio como todo comando de
/// Unix termina. Este app lista commits e repositórios — a saída que mais se lê com `head`.
#[cfg(unix)]
fn restaurar_sigpipe() {
    // SAFETY: chamada única, antes de qualquer thread, restaurando o handler padrão do SO.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}
#[cfg(not(unix))]
fn restaurar_sigpipe() {}

fn main() {
    restaurar_sigpipe();
    if let Err(e) = cli::executar(Cli::parse().cmd) {
        // Piso 4: erro NUNCA é engolido. Vai para o stderr, e o código de saída é 1 — quem
        // encadeia este comando num script precisa dos dois.
        eprintln!("erro: {e}");
        std::process::exit(1);
    }
}
