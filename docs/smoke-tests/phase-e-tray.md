# Phase E — Smoke tests manuais

Roteiro pra validar Phase E (`bl-tray` + `bl tray` + `bl report --copy`)
antes de mergear `phase-e-tray` para `main`. Esse smoke é o **merge gate**:
mesmo com `cargo test` 100% verde, **não mergear** até esse arquivo passar
e o usuário dar OK explícito.

> Pré-requisito GNOME: instale a extensão "AppIndicator and KStatusNotifierItem
> Support" (https://extensions.gnome.org/extension/615/appindicator-support/).
> Sem ela, o ícone simplesmente não aparece — não é regressão do bl-tray.
> KDE Plasma, XFCE, Cinnamon e MATE já mostram o ícone out of the box.

---

## Setup

```bash
cd /home/nuuvem/Projects/study/buckland
cargo build --release --features tray

alias bl="$PWD/target/release/bl"
alias bl-tray="$PWD/target/release/bl-tray"

export BUCKLAND_HOME=/tmp/bl-phase-e-smoke
rm -rf "$BUCKLAND_HOME"
```

> **Importante — `BUCKLAND_HOME` precisa ser exportado em TODO shell que
> rodar `bl` ou `bl-tray`.** Env vars não atravessam terminais. Se você
> abrir um segundo terminal para algum passo abaixo, repita o `export`
> antes; senão o `bl-tray` lê o DB default e o tooltip nunca acha o
> timer que `bl start` criou no outro shell. Confira com
> `echo $BUCKLAND_HOME` se desconfiar.

Confira que `wl-copy` (Wayland) ou `xclip` (X11) está instalado:

```bash
which wl-copy || which xclip || echo "FAIL: instale wl-clipboard ou xclip"
```

---

## 1. `bl-tray` sem DB — "no database yet"

- [ ] Sem nada em `$BUCKLAND_HOME` ainda, rode `bl-tray &` num terminal.
- [ ] O ícone aparece na bandeja como um relógio **cinza** (idle).
- [ ] Hover sobre o ícone — tooltip diz `Buckland: no database yet`.
- [ ] Process não morre, fica idle.

(Mantenha `bl-tray` rodando para os próximos cenários.)

---

## 2. Polling detecta criação do banco e timer ativo

Em outro terminal (**lembre de re-exportar** `BUCKLAND_HOME=/tmp/bl-phase-e-smoke`):

```bash
export BUCKLAND_HOME=/tmp/bl-phase-e-smoke
alias bl="$PWD/target/release/bl"   # se necessário; senão use o path completo

bl add "fix login flow"
bl add "refactor imports"
bl start 1
```

- [ ] Dentro de `tray.poll_seconds` (default 2s — espere até 5s pra
      garantir margem), o tooltip muda pra
      `#1 fix login flow — 00:00:XX (started HH:MM)`.
- [ ] O ícone troca pra **verde** (running) — mesmo desenho de relógio,
      cor diferente.
- [ ] Hover repetido em segundos consecutivos — o `XX` avança 1, 2, 3...
      mostrando que o tick local de 1Hz está vivo (não dependente de poll).
- [ ] `bl stop`. Dentro do próximo poll, ícone volta pra **cinza** e
      tooltip vira `Buckland: idle`.

---

## 3. SC-prefix vs hash-prefix no tooltip

Configure um token de Shortcut válido em `$XDG_CONFIG_HOME/buckland/config.toml`
(ou pule esta seção se não tiver token agora):

```bash
bl add "linked task" --sc 4242   # cacheia a story 4242
bl start 3                        # task #3 com SC linkada
```

- [ ] Tooltip mostra `SC-4242 linked task — 00:00:XX (started HH:MM local)`.
- [ ] `bl stop`. `bl start 1` (sem SC). Tooltip volta ao formato `#1 ...`.

---

## 4. Restart da mesma task troca o ícone

- [ ] Com timer rodando (qualquer task), `bl start 2` no shell.
- [ ] Dentro do próximo poll: ícone faz visual flicker (transição
      detectada), tooltip atualiza pro task #2 com `started_at` novo.

---

## 5. Erro de leitura — ícone error

Simule um banco inacessível:

```bash
chmod 000 "$BUCKLAND_HOME/buckland.db"
```

- [ ] Próximo poll: ícone troca pra **vermelho** com `!` no meio (error).
      Tooltip: `Buckland: cannot read database — <razão truncada>`.

```bash
chmod 600 "$BUCKLAND_HOME/buckland.db"
```

- [ ] Próximo poll: tray recupera (idle ou active conforme estado real).

---

## 6. Menu com estado dinâmico + Quit

O `ubuntu-appindicators` no GNOME 49 não renderiza `Title`/`ToolTip` no
hover do painel, então o estado live mora no menu — re-renderizado a
cada tick de 1Hz.

- [ ] Right-click no ícone → menu tem **três coisas**:
      1. Primeira linha (cinza/disabled): texto igual ao do tooltip
         (ex. `#1 fix login flow — 00:01:23 (started 14:02)` quando
         rodando; `Buckland: idle` quando idle).
      2. Separador horizontal.
      3. `Quit`.
- [ ] Com timer rodando, abrir o menu duas vezes em segundos diferentes —
      o `XX` (segundos) da primeira linha avança entre as aberturas.
- [ ] Click em "Quit" — `bl-tray` termina cleanly (`echo $?` → 0).

---

## 7. `bl tray` (subcomando) é equivalente

- [ ] `bl tray` num terminal — comportamento idêntico ao `bl-tray`.
      Mesma nuance de poll, mesmo tooltip, mesmo menu.
- [ ] `Ctrl+C` no terminal fecha o tray (SIGINT). Sem ressaca no shell.
- [ ] `bl tray --help` exibe descrição "tray icon".

---

## 8. `bl report --copy` em Wayland

```bash
echo $WAYLAND_DISPLAY        # deve ser "wayland-0" ou similar
bl start 1; sleep 5; bl stop  # gera entry
```

- [ ] `bl report --copy` — stderr mostra `Copied to clipboard via wl-copy`.
      Stdout vazio. Exit 0.
- [ ] `wl-paste` cola exatamente uma linha tipo
      `buckland today — 5s across 1 row`.
- [ ] `bl report --copy --json` — `wl-paste` retorna um objeto JSON
      válido (começa com `{`, termina com `}`).

---

## 9. `bl report --copy` em X11

(Pule se não tiver acesso a uma sessão X11. Pode ser pulado em laptops
puramente Wayland.)

```bash
unset WAYLAND_DISPLAY
echo $DISPLAY                 # deve ser ":0" ou similar
```

- [ ] `bl report --copy` — stderr mostra `Copied via xclip`. Exit 0.
- [ ] `xclip -o -selection clipboard` retorna o one-liner.

---

## 10. `bl report --copy` sem display server

Em uma TTY pura (Ctrl+Alt+F3) ou via SSH sem `-X`:

```bash
unset WAYLAND_DISPLAY DISPLAY
bl report --copy
```

- [ ] Exit code 1. Stderr contém `clipboard copy failed: no display server detected`.
- [ ] Nada vai pro stdout.

---

## 11. TUI Report `c` (regressão Phase D)

```bash
bl tui
# r → Report → c
```

- [ ] Wayland: footer mostra `Copied via wl-copy`. `q` sai limpo.
- [ ] X11: `Copied via xclip`. Sem ambiente: `Copy failed: no display
      server detected (...)`. Sem panic.

---

## 12. `--no-default-features` build (regressão)

```bash
cargo build --no-default-features 2>&1 | tail -5
```

- [ ] Build verde. `target/debug/bl tray --help` retorna erro de
      subcomando desconhecido (clap), porque `Tray` está cfg-gated.
- [ ] `target/debug/bl-tray` **não existe** (binary requires `tray`).

---

## Sinais de alerta (qualquer um aborta o merge)

- Ícone não aparece em **nenhum** desktop testado (sem ser o caveat do GNOME).
- Tooltip mostra timestamp em UTC em vez de local.
- Tooltip avança o `XX` por menos de 1Hz ou trava.
- `bl-tray` consome CPU acima de ~1% em idle (poll mais 1Hz tick deve ser ruído).
- `bl tray` segura DB lock (rode `bl add "x"` com `bl tray` rodando — deve
  funcionar instantaneamente; se travar, há regressão de read-only mode).
- Menu do tray tem qualquer coisa diferente de: estado-disabled, separador, Quit.
- `bl report --copy` panica em qualquer ambiente em vez de exit code 1.
- `bl report --copy --json` cola uma tabela ASCII em vez de JSON.

---

## Cleanup

```bash
pkill -f bl-tray || true
rm -rf "$BUCKLAND_HOME"
unset BUCKLAND_HOME
unalias bl bl-tray
```
