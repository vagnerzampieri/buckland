# Phase B — Smoke tests manuais

Roteiro pra validar Phase B (Shortcut integration) antes de mergear para `main`.
Tudo roda com `BUCKLAND_HOME` isolado; não mexe na sua base real.

> Dica: abra este arquivo em um painel e o terminal em outro. Marque o checkbox
> de cada passo conforme for passando.

---

## Setup

**O setup precisa ser feito dentro da pasta do projeto Buckland** (`cargo
build` só funciona no diretório que tem o `Cargo.toml`). Depois que o alias
estiver definido com o caminho absoluto do binário, os comandos `bl ...`
funcionam de qualquer pasta — inclusive de `~` ou `/tmp`.

```bash
cd /home/nuuvem/Projects/study/buckland   # ou onde estiver o projeto
cargo build --release                      # precisa rodar aqui

# Alias pra ficar curto — `$PWD` expande AGORA pro path absoluto,
# então o alias continua válido depois de qualquer `cd`.
alias bl="$PWD/target/release/bl"

# Ambiente isolado (DB em /tmp, não toca ~/.local/share)
export BUCKLAND_HOME=/tmp/bl-phase-b-smoke
rm -rf "$BUCKLAND_HOME"

# A partir daqui você pode `cd` para qualquer lugar — `bl` e `BUCKLAND_HOME`
# são cwd-independentes. Exemplos:
cd ~                                       # ou `cd /tmp`, tanto faz
bl --help                                  # funciona
```

Alternativa sem alias (mas obriga a ficar no diretório do projeto):
`cargo run --quiet -- <args>`.

---

## 1. Regressão Phase A (não pode quebrar nada do que já funcionava)

- [ ] `bl add "fix login"` — imprime `Added: #1 fix login`
- [ ] `bl add "refactor imports"` — imprime `Added: #2 refactor imports`
- [ ] `bl list` — mostra as duas tarefas **sem coluna SC-id**
- [ ] `bl start 1` — imprime `Started: #1 fix login (HH:MM:SS)`
- [ ] `bl status` — exit **0**, mostra "fix login"
- [ ] `bl stop` — imprime `Stopped: #1 fix login (00:00:XX)`
- [ ] `bl status` — exit **1**, "No active timer."
- [ ] `bl done 1` — imprime `Done: #1 fix login`
- [ ] `bl list` — só a #2 aparece (completed está oculto)
- [ ] `bl delete 2` — se #2 nunca foi iniciada, exclui; se tem time entry, bloqueia

---

## 2. `--sc` sem token (deve falhar de forma clara, nunca panicar)

Sem `~/.config/buckland/config.toml` configurado.

- [ ] `bl add "x" --sc SC-123` — exit **1**, mensagem: `shortcut.token is not configured in config.toml`
- [ ] Confirma: `bl list --all` **não** mostra "x" (nada foi persistido)
- [ ] `bl shortcut SC-1` — exit **1**, mesma mensagem
- [ ] `bl start SC-1` — exit **1**, mesma mensagem
- [ ] `bl list --all` — ainda **não** existe tarefa vinculada a SC-1

---

## 3. IDs inválidos (rejeitados antes de qualquer chamada HTTP)

- [ ] `bl add "x" --sc ABC-1` — exit **1**, `invalid shortcut id: ABC-1`
- [ ] `bl add "x" --sc ""` — exit **1** (clap já rejeita vazio, ou resolver)
- [ ] `bl add "x" --sc SC-` — exit **1**, `invalid shortcut id: SC-`
- [ ] `bl add "x" --sc 0` — exit **1**
- [ ] `bl add "x" --sc -5` — exit **1**, `invalid shortcut id: -5`
- [ ] `bl list --all` — **nenhuma** tarefa foi criada nos passos acima

---

## 4. Bare-numeric: task-id tem precedência sobre story-id

Mesmo sem token configurado, bare "1" que casa com `tasks.id` resolve local.

- [ ] `rm -rf "$BUCKLAND_HOME"` (reset)
- [ ] `bl add "primeiro"` — cria task #1
- [ ] `bl start 1` — inicia task #1, **sem ir na rede** (não precisa de token)
- [ ] `bl status` — mostra "primeiro"
- [ ] `bl stop`

---

## 5. Com token real (opcional — requer acesso ao Shortcut)

Pule esta seção se não quiser bater na API real. Os testes integrados
(`cargo test --test cli_shortcut_add` etc.) já cobrem todos os caminhos
contra mockito.

### 5a. Configurar token

```bash
mkdir -p ~/.config/buckland
cat > ~/.config/buckland/config.toml <<EOF
[shortcut]
token = "$SEU_TOKEN_SHORTCUT"
EOF
chmod 600 ~/.config/buckland/config.toml
```

- [ ] `stat -c "%a" ~/.config/buckland/config.toml` — imprime **600**

Escolha um SC-ID real do seu workspace; vou chamar de **SC-X** abaixo.
Sugestão: pegue uma story sua que esteja ativa.

### 5b. `bl shortcut` força refresh do cache

- [ ] `bl shortcut SC-X` — exit **0**, imprime `SC-X <título da story> — fetched_at YYYY-MM-DD HH:MM:SS`
- [ ] Rodar de novo — deve funcionar igual (mesmo não sendo forçado, refresh sempre vai à rede)

### 5c. `bl add --sc` cria tarefa linkada

- [ ] `bl add "trabalho local" --sc SC-X` — exit **0**, imprime `Added: #N trabalho local (SC-X)`
- [ ] `bl list` — agora aparece **coluna SC-id**; a tarefa "trabalho local" mostra `SC-X`

### 5d. `bl start SC-X` retoma (não duplica)

- [ ] `bl start SC-X` — exit **0**, inicia a tarefa #N existente
- [ ] `bl status` — mostra o título da tarefa
- [ ] `bl stop`
- [ ] `bl list --all | grep -c SC-X` — imprime **1** (apenas uma tarefa linkada)

### 5e. `bl start SC-NOVO` cria + linka + inicia tudo de uma vez

Use outro SC-ID real que **ainda não esteja** vinculado a nenhuma tarefa local.

- [ ] `bl start SC-NOVO` — exit **0**, imprime `Started: #M <título da story> (HH:MM:SS)`
- [ ] `bl list` — duas tarefas, ambas com `SC-id`
- [ ] `bl stop`

### 5f. 404 não cria tarefa fantasma

Use um ID improvável (ex. `SC-99999999` que não existe no seu workspace).

- [ ] `bl start SC-99999999` — exit **1**, `shortcut story SC-99999999 not found`
- [ ] `bl list --all | grep -c 99999999` — imprime **0** (nenhuma tarefa foi persistida)

---

## 6. Histórico segue protegido

- [ ] `bl start "temporário"` — cria task e inicia timer
- [ ] `bl stop`
- [ ] `bl delete <id-dessa-task>` — exit **1**, sugere archive
- [ ] `bl archive <id-dessa-task>` — exit **0**
- [ ] `bl list` — não aparece mais
- [ ] `bl list --archived` — aparece

---

## Sinais de alerta durante os testes

Se qualquer um destes acontecer, **não mergear** e reportar:

- `panic!`, `unwrap`, ou backtrace no stderr
- Mensagem de erro crua exposta (tipo `reqwest::Error { url: ..., source: ... }` sem tratamento)
- `bl list` mostrando coluna SC-id quando nenhuma tarefa tem story vinculada
- Token aparecendo em qualquer output (stdout/stderr)
- Tarefa sendo criada quando o fetch falha em `bl add --sc`
- `bl start SC-X` criando tarefa duplicada se rodado duas vezes
- Lentidão anormal em operações que não deveriam ir à rede (bare-numeric que casa task-id, `bl list`, etc.)

---

## Cleanup

```bash
unset BUCKLAND_HOME
unalias bl 2>/dev/null
rm -rf /tmp/bl-phase-b-smoke
```

O `~/.config/buckland/config.toml` fica (é a sua config real pra próximas fases).
