# Phase D — Smoke tests manuais

Roteiro pra validar Phase D (ratatui TUI) antes de mergear para `main`.
Tudo roda com `BUCKLAND_HOME` isolado; não mexe na sua base real. Como o
TUI é interativo, cada passo é uma tecla que você aperta + o que deve
aparecer na tela.

> Dica: abra este arquivo num painel e o terminal noutro. O TUI pega a tela
> inteira; mantenha o markdown visível em outra janela.

---

## Setup

```bash
cd /home/nuuvem/Projects/study/buckland   # ou onde estiver o projeto
cargo build --release

alias bl="$PWD/target/release/bl"

export BUCKLAND_HOME=/tmp/bl-phase-d-smoke
rm -rf "$BUCKLAND_HOME"

# Seed mínimo via CLI (Phase A/B/C inalterados)
bl add "fix login flow"
bl add "refactor imports"
bl start 1
sleep 4
bl stop                                    # cria uma time entry de ~4s na #1
```

---

## 1. Launch sanity

- [ ] `bl tui` — abre fullscreen alt-screen, header mostra "Buckland — idle"
      (timer parado), tela Tasks listando #1 e #2, footer com hints.
- [ ] Apertar `q` — terminal volta ao prompt **sem lixo** (sem cores presas,
      cursor visível, prompt limpo). Exit code 0.
- [ ] `bl` (sem subcomando) — abre o mesmo TUI. `q` sai limpo.

---

## 2. Tasks screen — navegação

Abra o TUI (`bl tui`). Ainda em Tasks:

- [ ] `j` move o highlight pra baixo, `k` pra cima — clamp nas pontas (não
      cycla).
- [ ] `gg` (dois `g` rápidos) volta pro topo; `G` vai pro fim.

---

## 3. Tasks screen — start / stop

- [ ] `s` na #1 — header passa a mostrar `▶ fix login flow — 00:00:0X`
      (relógio piscando lento via SLOW_BLINK). Footer: "Started #1 ...".
- [ ] Esperar 2s. Header continua atualizando o HH:MM:SS sozinho (1Hz tick).
- [ ] `S` — header volta pra "idle". Footer: "Stopped".
- [ ] `s` em #2 com #1 ainda rodando: aciona stop+start atômico — header
      passa a mostrar #2, #1 vira histórico. (Pra reproduzir esse caso,
      reinicie #1 antes com `s`.)

---

## 4. Tasks screen — new task (n) inline

- [ ] `n` abre prompt no rodapé `New task: _`.
- [ ] Digite `nova story`, `Enter` — task #3 aparece no topo da lista.
      Footer: "Added #3 nova story".
- [ ] `n`, digite `xx`, `Esc` — prompt fecha sem criar nada.
- [ ] `n`, `Enter` (vazio) — footer "Empty title — nothing created.".

---

## 5. Tasks screen — done / archive / delete

- [ ] `d` numa task aberta — sai da lista (default esconde completed).
      Footer "Done #N ...".
- [ ] `A` numa task aberta — também some. Footer "Archived #N ...".
- [ ] `D` numa task **sem** time entries — abre prompt `Delete task #N "..." y/N`.
      `y` deleta; `n` (ou qualquer outra tecla) cancela com "Cancelled".
- [ ] `D` numa task **com** time entries (a #1 do seed tem) — `y` resulta em
      footer **vermelho** com "Task #N has time entries — use Archive (A)
      instead.". A task **NÃO** some.

---

## 6. Tasks screen — filter (/)

- [ ] `/` abre prompt `Filter: _`.
- [ ] Digite `log`, `Enter` — só tasks com "log" no título ficam visíveis.
- [ ] `/`, `Esc` — limpa o filtro existente, lista volta completa.

---

## 7. Agenda screen

- [ ] `a` troca pra Agenda. Título mostra "Agenda — week of YYYY-MM-DD"
      (segunda-feira da semana atual local). Entry do seed (~4s na #1)
      aparece sob o header do dia formatado tipo "Wed 29 Apr".
- [ ] `h` paginação pra semana anterior — entries somem (semana vazia
      mostra "(no entries this week)"). `l` volta.
- [ ] `j`/`k` navegam entre entries quando há mais de uma.
- [ ] `Enter` numa entry abre a Edit overlay (próxima seção).
- [ ] `D` numa entry abre prompt `Delete entry #N (Xs)? y/N`. `y` deleta,
      footer "Deleted entry #N".

---

## 8. Edit overlay (a partir da Agenda)

Com Agenda aberta e entry selecionada, `Enter`:

- [ ] Modal centralizado com backdrop dim (resto da tela escurecido).
      Mostra `Task: <título>` (read-only) + 3 campos (Started, Ended, Notes).
- [ ] Started/Ended formatados `YYYY-MM-DD HH:MM` em horário **local**.
- [ ] `Tab` cicla foco: Started → Ended → Notes → Started. Campo focado
      destacado em REVERSED.
- [ ] Digite caracteres no campo focado — buffer aparece. `Backspace` apaga.
- [ ] `Enter` salva: modal fecha, footer "Saved entry #N", Agenda reflete
      mudança.
- [ ] Reabrir e botar `not a date` no Started, `Enter` — footer **vermelho**
      "Invalid started_at: not a date", modal continua aberto.
- [ ] `Ctrl+D` — footer do modal vira `Delete this entry? y/N`. `n`
      cancela, `y` deleta a entry e fecha o modal.
- [ ] `Esc` (ou `q`) — modal fecha sem salvar, footer "Cancelled" não
      aparece (Esc na overlay só fecha).

---

## 9. Report screen

- [ ] `r` abre Report. Título: `Report — Today / by task`. Linha por task
      do dia + barra Unicode (`█▏▎...`) + total no rodapé "Total".
- [ ] `Tab` cicla escopo Today→Week→Month→All→Today. Título atualiza.
- [ ] `T` cicla agrupamento by task→by epic→by day→by task. Linhas mudam.
- [ ] `j` (resolve a `Down`, repurpose dessa tela) toggla JSON dump — tela
      vira um JSON `serde_json::to_string_pretty` do report. `j` de novo
      volta pra tabela.
- [ ] `c` copia one-liner pro clipboard:
  - Wayland (`$WAYLAND_DISPLAY` setado): footer "Copied via wl-copy". Cole
    em outra app pra confirmar.
  - X11: "Copied via xclip". Idem.
  - Sem nenhum dos dois instalados: footer **vermelho** "Copy failed: no
    clipboard tool found (need wl-copy or xclip)".

---

## 10. Help overlay

- [ ] `?` abre Help. Categorias visíveis: Navigation, View, Tasks, Report,
      Edit overlay. Cada seção em **bold**.
- [ ] Qualquer tecla (exceto `q`/`Esc`) volta pra Tasks. `q`/`Esc` também
      fecha (mas via global handler, sem quitar o app).
- [ ] `?` abre Help, `q` — volta pra Tasks (NÃO sai do TUI).

---

## 11. State persistente entre telas

Com timer rodando (após `s`):

- [ ] `g`→Tasks, `a`→Agenda, `r`→Report, `?`→Help — header continua
      mostrando `▶ <task> — HH:MM:SS` em **todas** as telas e o relógio
      continua atualizando.
- [ ] `S` (em qualquer tela que aceite, ou volte pra Tasks pra apertar) —
      timer para; header vira "idle" instantaneamente.

---

## 12. Persistência via SQLite

- [ ] No TUI, criar uma task nova com `n nova via tui`. Sair com `q`.
- [ ] `bl list` no shell — a task aparece, persistida.
- [ ] `bl tui` de novo — task ainda lá.

---

## Sinais de alerta (qualquer um aborta o merge)

- Terminal sai sujo (cores presas, cursor invisível, prompt em raw mode).
- Painc visível em vez de erro tratado no footer.
- Timer ativo desaparece do header sem o usuário ter apertado `S`.
- `q` em uma overlay (Help, Edit, prompt) sai do app inteiro em vez de só
  fechar a overlay.
- `gg` em vez de jumpar pro topo dispara duas trocas de tela ou trava.
- Conteúdo escrito no Edit não persiste (re-abrir mostra valor antigo).
- Clipboard `c` panica em ambiente sem wl-copy nem xclip — deve apenas
  surfar erro no footer.

---

## Cleanup

```bash
rm -rf "$BUCKLAND_HOME"
unset BUCKLAND_HOME
unalias bl
```
