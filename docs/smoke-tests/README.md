# Smoke tests manuais

Checklists reprodutíveis pra validar cada fase antes de mergear para `main`.
Complementam os testes automatizados (`cargo test`) com a parte que só um
humano pilota: ergonomia de mensagens de erro, saída formatada, integrações
reais, e o famoso "rodei o binário inteiro numa sessão e nada quebrou".

## Quando rodar

- **Antes** de mergear uma feature branch de uma fase pra `main`.
- Antes de cortar uma release no GitHub (depois do merge).
- Opcionalmente, depois de upgrades grandes de dependências.

Os testes automatizados continuam sendo a primeira linha de defesa. Este
diretório existe pra pegar o que `cargo test` não pega: UX, outputs
formatados, e integrações com serviços reais (Shortcut, futuramente tray DBus).

## Como rodar

Cada arquivo é auto-contido. Abra o `.md` da fase em um painel, terminal no
outro, e marque os checkboxes (`- [ ]` → `- [x]`) conforme passar. Se algo
falhar, **não mergear** — reporte o comportamento observado.

## Convenção

- Um arquivo por fase: `phase-<letra>-<slug>.md`.
- Começa com um **Setup** isolando estado em `/tmp` via `BUCKLAND_HOME`.
- Seções numeradas com objetivo explícito ("Regressão Phase A", "IDs inválidos",
  "Com token real", etc.).
- Cada passo tem **comando + output esperado**, não só o comando.
- Seções que dependem de credenciais reais ficam explicitamente marcadas como
  opcionais.
- Termina com **Sinais de alerta** (o que NÃO deveria acontecer) e **Cleanup**.

## Arquivos

| Fase | Arquivo | Status |
|------|---------|--------|
| B — Shortcut integration | [`phase-b-shortcut.md`](phase-b-shortcut.md) | ativo |

Adicione fases futuras linkadas acima à medida que forem entregues.
