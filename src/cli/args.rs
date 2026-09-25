//! A superfície da CLI.
//!
//! **Onde:** `main`, na raiz da árvore do clap.
//!
//! **Os nomes vieram do `schematize git <sub>` do hub**, de propósito (E2 do ADR-0018): quem já
//! digitava `git status` digita `schematize-git status`. Mudar a forma do comando no mesmo dia
//! em que ele muda de dono cobraria duas mudanças de quem usa.

use clap::{Parser, Subcommand};

/// `schematize-git` — quem faz push, de onde, e com qual identidade.
#[derive(Parser)]
#[command(
    name = "schematize-git",
    // `version = <fn>` e nao `version` puro: o numero sozinho nao distingue dois binarios com o
    // mesmo `Cargo.toml` e comportamento diferente. Ver `nucleo/procedencia.rs`.
    version = git_casa::nucleo::procedencia::rotulo_versao(),
    about = "schematize git — accounts, repositories, and what has not left this machine yet",
    long_about = "More than one git identity on the same machine, without committing with the \
                  wrong one.\n\
                  Works standalone; the schematize hub delegates its Git tab to it."
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// List the registered accounts.
    Accounts {
        /// Machine-readable output (the window reads this).
        #[arg(long)]
        json: bool,
    },
    /// Register (or replace) an account.
    Add {
        /// Short label, no spaces ("pessoal", "trabalho").
        rotulo: String,
        #[arg(long)]
        usuario: String,
        #[arg(long)]
        email: String,
        /// Key file in ~/.ssh. Without it, the account authenticates through `gh`.
        #[arg(long)]
        chave: Option<String>,
        /// Service host (default github.com).
        #[arg(long)]
        servico: Option<String>,
    },
    /// DETECT accounts already on this machine (`gh`, git config, ~/.ssh, repo e-mails).
    ///
    /// It only suggests. `--add` is what writes — a detector that registered on sight would
    /// turn a guess into a fact.
    Detect {
        /// Register the suggestions that are not yet known.
        #[arg(long)]
        add: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Remove an account by label.
    Remove { rotulo: String },
    /// Apply an account to the repository in the current directory.
    Use {
        rotulo: String,
        /// Remote name (default origin).
        #[arg(long)]
        remoto: Option<String>,
    },
    /// Write the account's SSH alias into ~/.ssh/config.
    SshConfig { rotulo: String },
    /// List the account's repositories on the service (through `gh`).
    Repos {
        /// Only this account (default: all of them).
        rotulo: Option<String>,
        #[arg(long, default_value_t = 50)]
        limite: usize,
        #[arg(long)]
        json: bool,
    },
    /// What has NOT left this machine yet, project by project.
    Status {
        /// Scan this directory instead of the registered dev dirs. Repeatable.
        #[arg(long = "dir")]
        dirs: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Commits of the current project, marking the ones already pushed.
    Log {
        #[arg(long, default_value_t = 20)]
        limite: usize,
        #[arg(long)]
        json: bool,
    },
}
