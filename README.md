# schematize-git

**Quem faz push, de onde, e com qual identidade.** Mais de uma conta de git na mesma máquina,
sem commitar com a errada.

```
schematize-git detect                 # o que já existe aqui: gh, git config, ~/.ssh, repos
schematize-git detect --add           # cadastra o que ainda não está cadastrado
schematize-git accounts               # as contas registradas, e se o alias SSH está no lugar
schematize-git ssh-config pessoal     # escreve o alias no ~/.ssh/config
schematize-git use pessoal            # aplica a conta AO REPOSITÓRIO do diretório atual
schematize-git status                 # o que ainda NÃO saiu da máquina, projeto a projeto
schematize-git log                    # commits do projeto, marcando os já enviados
```

## O problema que ele resolve

Duas contas no mesmo computador — pessoal e de trabalho, ou duas organizações — e o git não tem
nada que impeça o commit de sair com a identidade errada. Quando sai, o commit já está no
histórico, e reescrevê-lo é caro.

**O truque é o alias SSH.** Em vez de apontar o remoto para `github.com`, cada conta ganha um
alias (`github.com-pessoal`), e o `~/.ssh/config` mapeia cada alias para a chave certa. Assim **o
remoto carrega consigo qual identidade usar** — não há estado global para errar.

`use` faz as duas pontas: escreve `user.name`/`user.email` **locais** no repositório e reaponta o
remoto para o alias da conta.

## `status` responde o que o git não responde

`git status` fala de um repositório. A pergunta que importa é outra: **de tudo que está nesta
máquina, o que ainda não saiu?** — e ela atravessa todos os projetos. É a pergunta que se faz
antes de formatar o disco, trocar de máquina, ou sair de férias.

## Por que é um app (ADR-0018, fase E2)

**Este foi o achado do inventário de extradição — não estava em ADR nenhum.** 64 unidades dentro
do app principal resolvendo um problema que nada tem a ver com skill, overdev ou instalação.

O domínio é fechado: a única coisa que ele importava do hub eram **quatro funções de plataforma**
(HOME, diretório de dados, leitura segura, execução de comando). Vieram junto, em
`src/nucleo/util.rs` — copiá-las é mais barato que inverter a dependência, e nenhuma delas sabe o
que é uma conta de git.

**Ele sobe sozinho** (piso 10). A ausência dele não impede o hub de bootar.

## O que ele NÃO guarda

**Nenhum segredo.** O cadastro tem rótulo, usuário, e-mail, host e *o nome do arquivo* da chave.
O que autentica é o `gh` ou a chave em `~/.ssh` — e nenhum dos dois passa por aqui.

## O `--json` é contrato

Os cinco comandos de leitura têm `--json`, e o documento é escrito à mão, sem `derive`. Há dois
motivos, e o segundo é específico deste app: o `Conta` **também** é serializado para o arquivo de
cadastro em disco. Se o mesmo `derive` servisse aos dois, o formato em disco e o contrato da
janela ficariam amarrados — e renomear um campo por clareza interna quebraria a janela.

`detect --json` **nunca grava**, nem com `--add`: quem pede um documento de máquina está
perguntando o estado, e a janela o chama a cada atualização de tela.

## Estado

Repo novo (E2 do ADR-0018). **Ainda não publicado no GitHub** — a pendência está declarada no
`.schematize/overdev/BLOCKED` do projeto, com o comando de publicação.

Falta a janela (M4) e a delegação da aba do hub (M5). Enquanto o hub ainda tiver a cópia, as duas
existem — e o inventário de extradição conta as duas, que é o estado real e não o desejado.
