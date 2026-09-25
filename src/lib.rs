//! **schematize-git** — quem faz push, de onde, e com qual identidade.
//!
//! **O quê:** as contas de git/GitHub da máquina, qual delas vale em cada repositório, os
//! repositórios do serviço, o estado local de cada projeto, e o histórico do que já foi enviado.
//!
//! **Onde:** o binário `schematize-git` e a janela dele.
//!
//! ## Por que é um app (ADR-0018, fase E2)
//!
//! **Este foi o achado do inventário de extradição — não estava em ADR nenhum.** 64 unidades no
//! app principal resolvendo um problema que nada tem a ver com skill, overdev ou instalação:
//! *ter mais de uma identidade git na mesma máquina e não commitar com a errada*.
//!
//! O domínio é fechado. A única coisa que ele importava do hub eram **quatro funções de
//! plataforma** (HOME, diretório de dados, leitura segura, execução de comando), que vieram
//! junto em [`nucleo::util`] — copiá-las é mais barato que inverter a dependência, e elas não
//! sabem o que é uma conta.
//!
//! O piso 10 vale: este app sobe sozinho, e a ausência dele não impede o hub de bootar.
//!
//! ## A divisão
//!
//! [`contas`] guarda o cadastro (**sem segredo** — o que autentica é o `gh` ou a chave SSH, e
//! nenhum dos dois passa por aqui); [`deteccao`] descobre as contas que já existem na máquina;
//! [`aplicar`] escreve a identidade no repositório; [`repos`] fala com o serviço e resume o
//! estado local; [`historico`] lê o `git log` e o upstream.

pub mod aplicar;
pub mod contas;
pub mod deteccao;
pub mod historico;
pub mod nucleo;
pub mod repos;

pub use contas::{Auth, Conta};
pub use deteccao::{Origem, Sugestao};
pub use historico::{Commit, Upstream};
pub use repos::{EstadoLocal, Remoto};
