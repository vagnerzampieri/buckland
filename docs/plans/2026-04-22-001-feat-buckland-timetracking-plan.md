---
title: "feat: Buckland time-tracking — Rails core + Rust quick-access"
type: feat
status: active
date: 2026-04-22
origin: docs/claude/plan/buckland.md
---

# Buckland time-tracking — Rails core + Rust quick-access

## Overview

Buckland é uma aplicação pessoal de time-tracking com integração ao Shortcut, composta por dois binários:

1. **Aplicação principal (Rails + Hotwire)** — fonte da verdade para tarefas, sessões de tempo, agenda e relatórios. Expõe UI web rica para dados densos e uma API HTTP JSON usada pelo quick-access.
2. **Quick-access nativo (Rust + gtk-rs)** — app de bandeja/tray no Linux com ações rápidas (start/stop/listar ativos), consumindo a API local do Rails.

O objetivo primário é ter fricção quase zero pra iniciar um timer ligado a uma story do Shortcut, e uma visão consolidada do que foi feito depois.

## Problem Frame

O brief (`docs/claude/plan/buckland.md`) descreve:

- Contar tempo gasto em tarefas, integrando com Shortcut.
- Todo-list que usa o timer.
- Agenda agrupada e relatórios.
- Start/Stop/Delete como operações primárias.
- Stack a decidir junto — decidida neste plano como Rails + Rust/gtk-rs (ver Key Technical Decisions).

O usuário-alvo é o próprio autor (projeto pessoal), focado em fluxo de trabalho de dev que já opera com Shortcut. A tensão principal é: precisa ser rápido para registrar (daí o tray nativo) e, ao mesmo tempo, permitir visualização densa (daí a app web completa).

## Requirements Trace

- **R1** — Operações de timer: start, stop e delete sobre uma tarefa (`origin: brief § Funcionalidades`).
- **R2** — Integração Shortcut: dado um ID da story, puxar dados do Shortcut para pré-preenchimento (`origin: brief § Shortcut`).
- **R3** — Todo-list: criar/listar tarefas facilmente, cada item pode iniciar um timer (`origin: brief § Todo-list`).
- **R4** — Agenda: visualização agrupada do que foi feito (`origin: brief § Funcionalidades`).
- **R5** — Relatórios: agregações sobre tempo registrado (`origin: brief § Overview`).
- **R6** — Acesso rápido nativo: disparar start/stop sem abrir o browser (`origin: conversa de planejamento — decisão do usuário`).

## Scope Boundaries

- Single-user — sem autenticação multiusuário, sem organizações/times. Vale localhost ou servidor pessoal.
- Somente Linux como alvo do quick-access (gtk-rs, AppIndicator). Sem build macOS/Windows.
- Shortcut integration é **read-only** — puxa dados de uma story por ID; não publica time entries nem muda status da story.
- Timer **único ativo por vez** — iniciar um timer novo para automaticamente o ativo anterior. Sem pomodoro, sem múltiplos timers paralelos.
- Sem sincronização multi-dispositivo no v1. O banco vive num só lugar (máquina do usuário ou VPS pessoal).

### Deferred to Separate Tasks

- Write-back para Shortcut (registrar time entries na story): avaliar depois de usar o v1 por algumas semanas.
- Empacotamento (.deb / AppImage / Flatpak) do quick-access para distribuição: fazer num plano próprio quando estabilizar.
- Deploy do Rails (além de `bin/dev` local): decidir host (Fly.io, self-hosted) quando surgir necessidade real de acessar de outra máquina.

## Context & Research

### Relevant Code and Patterns

Greenfield — não há código pré-existente no repo. Padrões a seguir vêm das convenções da stack:

- **Rails 8+ defaults**: SQLite + Solid Queue + Solid Cache + Hotwire (Turbo + Stimulus) + Propshaft.
- **Tailwind CSS** via `tailwindcss-rails` (watcher integrado ao `bin/dev`). Dark mode `prefers-color-scheme`. Sem tema custom no v1.
- **Minitest** como framework de teste (padrão Rails). RSpec não é necessário para projeto pessoal.
- **Service objects** para integração com Shortcut (Plain Old Ruby Object em `app/services/`).
- **ViewComponent** pode ser adotado para agenda/timer widget, mas não é obrigatório no v1 — Hotwire partials dão conta.
- **gtk-rs + libadwaita-rs** para o quick-access, seguindo convenções de app GNOME (Adwaita widgets, `.ui` files via Blueprint ou XML, `GResource` para assets).
- **reqwest** ou `ureq` para HTTP no Rust. `serde_json` para parsing.
- **ksni** para ícone na bandeja do sistema (StatusNotifierItem, compatível com GNOME/KDE).

### Institutional Learnings

- Sem `docs/solutions/` neste repo (greenfield). À medida que problemas forem resolvidos durante a execução, capturar aprendizados lá.
- Conhecimento do autor: domínio forte de Rails; Rust em nível iniciante-intermediário (declarado na conversa de planejamento).

### External References

- Shortcut API: `https://developer.shortcut.com/api/rest/v3` — endpoints relevantes: `GET /stories/{id}`, `GET /epics/{id}`, auth via header `Shortcut-Token`.
- gtk-rs book: `https://gtk-rs.org/gtk4-rs/stable/latest/book/`
- `ksni` crate docs para tray icon Linux.

## Key Technical Decisions

- **Dois componentes, um banco de dados**: Rails é o único dono dos dados. O Rust app é cliente HTTP. Evita sincronização dupla e simplifica consistência.
  - *Rationale*: complexidade mínima, single source of truth, facilita iteração. A latência de uma chamada HTTP local (~5ms) é imperceptível para o usuário.
- **API JSON local em `localhost:3000/api/...`**: sem token/auth no v1, protegida por `bind: 127.0.0.1` apenas.
  - *Rationale*: single-user em localhost; adicionar token quando (e se) deploy remoto for feito. Risco aceitável para projeto pessoal.
- **Timer state no banco**: `TimeEntry` com `started_at` e `ended_at` nullable. Entry ativa = `ended_at IS NULL`. Quick-access lista ativa via `/api/time_entries/active`.
  - *Rationale*: derivar estado do banco evita estado in-memory que se perde entre restart do Rails/tray. Index em `(ended_at)` partial para query rápida.
- **Shortcut PAT em ENV**: token pessoal em `ENV['SHORTCUT_API_TOKEN']`, não exposto via UI.
  - *Rationale*: credenciais de API não pertencem a UI/admin num app single-user. Commit-safe via `.env.example`.
- **Minitest + fixtures**: padrão Rails 8, sem introdução de RSpec ou FactoryBot.
  - *Rationale*: projeto pessoal, manter stack minimalista e alinhada com defaults.
- **Rust app sem UI state duplicado**: o tray app faz polling leve (a cada 10s) do endpoint `/active` ou, futuramente, SSE. Sem cache local.
  - *Rationale*: simplicidade. Polling é aceitável para granularidade de segundos num app pessoal; SSE pode substituir depois sem impacto na API.

## Open Questions

### Resolved During Planning

- Plataforma-alvo → Rails web + Rust gtk-rs (Linux).
- Escopo da integração Shortcut → read-only, pull-by-ID.
- Semântica do timer → único ativo por vez.
- Stack de testes → minitest (Rails), cargo test (Rust).
- Multi-user → não no v1.

### Deferred to Implementation

- **Formato exato dos relatórios** — quais agregações (por dia, semana, story, epic, projeto) e qual ordem de prioridade. A definir conforme o uso revelar necessidade.
- **Nome exato dos endpoints JSON** — seguir REST Rails-like (`/api/time_entries`, `/api/tasks`); ajustes finos durante implementação.
- **Tema do gtk-rs** — seguir Adwaita default; dark mode automático via sistema. Ajustes visuais a decidir quando o protótipo existir.
- **Armazenamento do Shortcut Story data** — cache em tabela dedicada vs. fetch on-demand: decidir ao implementar a Unit 4 baseado em como o Shortcut API se comporta (rate limit, latência).

## Output Structure

```
buckland/
├── Gemfile
├── config.ru
├── bin/
│   ├── dev
│   └── rails
├── app/
│   ├── controllers/
│   │   ├── tasks_controller.rb
│   │   ├── time_entries_controller.rb
│   │   ├── agenda_controller.rb
│   │   ├── reports_controller.rb
│   │   └── api/
│   │       ├── time_entries_controller.rb
│   │       └── tasks_controller.rb
│   ├── models/
│   │   ├── task.rb
│   │   ├── time_entry.rb
│   │   └── shortcut_story.rb
│   ├── services/
│   │   └── shortcut/
│   │       └── story_fetcher.rb
│   ├── views/
│   │   ├── tasks/
│   │   ├── agenda/
│   │   └── reports/
│   └── javascript/
│       └── controllers/
│           └── timer_controller.js
├── config/
│   ├── routes.rb
│   └── database.yml
├── db/
│   ├── migrate/
│   └── schema.rb
├── test/
│   ├── models/
│   ├── controllers/
│   ├── services/
│   └── fixtures/
└── quick-access/
    ├── Cargo.toml
    ├── src/
    │   ├── main.rs
    │   ├── api_client.rs
    │   ├── tray.rs
    │   └── ui/
    │       └── mini_window.rs
    ├── resources/
    │   └── icons/
    └── tests/
```

> *O layout é uma declaração de escopo, não restrição. A implementação pode ajustar se uma organização melhor emergir.*

## High-Level Technical Design

> *Ilustra a abordagem pretendida como guia de revisão, não especificação de implementação.*

```
┌───────────────────────┐          ┌─────────────────────────┐
│  Rust quick-access    │          │   Rails app (completo)  │
│  (gtk-rs + tray)      │          │                         │
│                       │          │  UI Web (Hotwire)       │
│  ┌─────────────────┐  │   HTTP   │  ┌────────────────┐    │
│  │ Tray icon +     │◄─┼──────────┼──│ TasksController │    │
│  │ mini window     │  │  JSON    │  │ AgendaController│    │
│  │ (start/stop/    │  │          │  │ ReportsController│   │
│  │  list active)   │  │          │  └────────────────┘    │
│  └────────┬────────┘  │          │         ▲               │
│           │           │          │         │               │
│  ┌────────▼────────┐  │          │  ┌──────┴──────┐         │
│  │ api_client.rs   │──┼──────────┼─►│ /api/...    │         │
│  │ (reqwest)       │  │          │  │ Controllers │         │
│  └─────────────────┘  │          │  └──────┬──────┘         │
└───────────────────────┘          │         │                │
                                   │  ┌──────▼──────┐         │
                                   │  │ Task /      │         │
                                   │  │ TimeEntry / │         │
                                   │  │ Shortcut    │         │
                                   │  │ models      │         │
                                   │  └──────┬──────┘         │
                                   │         │                │
                                   │  ┌──────▼──────┐         │
                                   │  │   SQLite    │         │
                                   │  └─────────────┘         │
                                   │         ▲                │
                                   │  ┌──────┴──────────┐     │
                                   │  │ Shortcut::      │     │
                                   │  │ StoryFetcher    │◄────┼── api.app.shortcut.com
                                   │  │ (service)       │     │
                                   │  └─────────────────┘     │
                                   └─────────────────────────┘
```

Fluxo típico "iniciar timer via tray":
1. Usuário clica no tray → mini window abre.
2. Digita `SC-123` no input → Rust faz POST `/api/time_entries` com `shortcut_id`.
3. Rails: encontra ou cria Task (via `Shortcut::StoryFetcher`), cria TimeEntry com `started_at`.
4. Resposta JSON volta; mini window fecha; tray mostra ícone "running" com tempo decorrido.
5. Polling a cada 10s atualiza o tempo exibido.

## Implementation Units

- [ ] **Unit 1: Bootstrap Rails app + modelos de domínio**

**Goal:** Subir uma app Rails 8+ funcional com os modelos `Task`, `TimeEntry` e `ShortcutStory` e seus relacionamentos, fixtures e testes de modelo.

**Requirements:** R1, R3

**Dependencies:** nenhuma

**Files:**
- Create: `Gemfile`, `config/database.yml`, `config/routes.rb`
- Create: `app/models/task.rb`, `app/models/time_entry.rb`, `app/models/shortcut_story.rb`
- Create: `db/migrate/20260422000001_create_tasks.rb`, `db/migrate/20260422000002_create_time_entries.rb`, `db/migrate/20260422000003_create_shortcut_stories.rb`
- Test: `test/models/task_test.rb`, `test/models/time_entry_test.rb`, `test/models/shortcut_story_test.rb`
- Test: `test/fixtures/tasks.yml`, `test/fixtures/time_entries.yml`

**Approach:**
- `rails new buckland --database=sqlite3 --css=tailwind` como base (Hotwire, Solid Queue e Tailwind já vêm ligados nos defaults do Rails 8+).
- Task: `title`, `description`, `shortcut_story_id` (nullable FK), `archived_at`. Validações: title presence.
- TimeEntry: `task_id`, `started_at`, `ended_at` (nullable), `notes`. Validações: `started_at` presence; `ended_at >= started_at` quando presente.
- ShortcutStory: `external_id` (unique), `title`, `state`, `epic_name`, `fetched_at`. Cache leve dos dados puxados da API.
- Invariante central: apenas uma `TimeEntry` com `ended_at IS NULL` por vez (enforce no modelo via validação + index partial único).

**Patterns to follow:**
- Rails guides `models` + `active_record_validations`.
- Índice parcial SQLite: `CREATE UNIQUE INDEX idx_single_active ON time_entries(ended_at) WHERE ended_at IS NULL;`.

**Test scenarios:**
- Happy path: criar Task válida com title salva.
- Happy path: TimeEntry pode ser criada com `started_at` e sem `ended_at` (timer ativo).
- Edge case: validação rejeita TimeEntry com `ended_at < started_at`.
- Edge case: criar uma segunda TimeEntry ativa para qualquer task deve falhar (índice único parcial).
- Happy path: associação Task `has_many :time_entries`; destroy em Task destrói TimeEntries.
- Happy path: Task `has_one :shortcut_story` opcional; Task sem story é válida.
- Integration: criação via fixture reflete o mesmo estado de objeto instanciado.

**Verification:**
- `bin/rails db:migrate && bin/rails test` passa.
- `bin/rails console` permite `Task.create!(title: "foo")` e `task.time_entries.create!(started_at: Time.current)`.

- [ ] **Unit 2: Core do timer + API JSON interna**

**Goal:** Expor endpoints para start, stop, delete de timers e listagem de entry ativa, consumíveis pelo quick-access Rust.

**Requirements:** R1, R6

**Dependencies:** Unit 1

**Files:**
- Create: `app/controllers/api/base_controller.rb`, `app/controllers/api/time_entries_controller.rb`, `app/controllers/api/tasks_controller.rb`
- Modify: `config/routes.rb` (namespace `:api`, `resources :time_entries`, `resources :tasks`)
- Create: `app/models/concerns/timer_operations.rb` (módulo com `start!`, `stop!` — concentrar regra de "um ativo só")
- Test: `test/controllers/api/time_entries_controller_test.rb`, `test/controllers/api/tasks_controller_test.rb`
- Test: `test/models/concerns/timer_operations_test.rb`

**Approach:**
- Rotas: `POST /api/time_entries` (inicia novo; para ativo automaticamente), `POST /api/time_entries/:id/stop`, `DELETE /api/time_entries/:id`, `GET /api/time_entries/active`, `GET /api/tasks`, `POST /api/tasks`.
- Bind: reforçar `127.0.0.1` apenas em `config/puma.rb` para o ambiente `development`.
- JSON responses leves: `{ id, task: { id, title }, started_at, ended_at }`.
- `TimerOperations#start!(task:)`: se houver ativo, chama `stop!` nele antes de criar o novo. Wrap em `ActiveRecord::Base.transaction`.
- Sem autenticação no v1 — comentário explícito no `Api::BaseController` documentando a decisão.

**Execution note:** Começar com teste de integração POST /api/time_entries que expressa o contrato (novo timer para o ativo anterior automaticamente) antes de implementar.

**Patterns to follow:**
- Rails guides `action_controller_overview` (API-only controllers) — usar `ActionController::API` como superclass do `Api::BaseController`.
- JSON serialization inline (`render json: ...`) sem ActiveModelSerializer no v1; Jbuilder opcional se crescer.

**Test scenarios:**
- Happy path: POST `/api/time_entries` com `task_id` retorna 201 e JSON da entry ativa.
- Integration: POST `/api/time_entries` enquanto já existe uma ativa fecha a anterior e abre nova na mesma resposta (verificar `ended_at` na antiga).
- Happy path: POST `/api/time_entries/:id/stop` define `ended_at` e retorna 200.
- Edge case: POST `/api/time_entries/:id/stop` numa entry já parada retorna 422.
- Happy path: GET `/api/time_entries/active` retorna a entry ativa ou 204 quando não há.
- Error path: DELETE `/api/time_entries/:id` remove e retorna 204; DELETE em id inexistente → 404.
- Error path: JSON inválido / params faltando → 422 com erro descritivo.
- Integration: criar TimeEntry via API reflete no banco e é visível na UI web (Unit 3).

**Verification:**
- `curl -X POST http://127.0.0.1:3000/api/time_entries -H "Content-Type: application/json" -d '{"task_id":1}'` cria entry.
- Apenas uma entry ativa em qualquer instante, conferido via `TimeEntry.where(ended_at: nil).count == 1`.

- [ ] **Unit 3: Todo-list + timer UI web (Hotwire)**

**Goal:** Interface web para criar/listar tarefas, iniciar/parar timer em cada item com atualização ao vivo do tempo decorrido.

**Requirements:** R1, R3

**Dependencies:** Unit 2

**Files:**
- Create: `app/controllers/tasks_controller.rb`, `app/controllers/time_entries_controller.rb`
- Create: `app/views/tasks/index.html.erb`, `app/views/tasks/_task.html.erb`, `app/views/tasks/_form.html.erb`
- Create: `app/javascript/controllers/timer_controller.js` (Stimulus: tick a cada 1s para exibir duração)
- Create: `app/javascript/controllers/application.js`, `app/javascript/controllers/index.js`
- Modify: `config/routes.rb` (root → tasks#index, resources :tasks, nested :time_entries)
- Test: `test/controllers/tasks_controller_test.rb`, `test/system/task_management_test.rb` (system test com Capybara)

**Approach:**
- Página principal: lista de tasks não-arquivadas, form inline para adicionar nova (Turbo Frame), botão Start/Stop em cada linha.
- Turbo Streams para atualizar UI quando timer muda de estado (sem full reload).
- Stimulus `timer_controller.js`: recebe `data-started-at` via atributo e atualiza texto a cada segundo usando `setInterval`. Clean up em `disconnect()`.
- Controllers web (não-API) usam CSRF e sessão normal Rails; API controllers (Unit 2) desabilitam CSRF.

**Patterns to follow:**
- Rails guides `working_with_javascript_in_rails` (Turbo + Stimulus).
- Padrão `turbo_stream` actions em response para `create`/`update`/`destroy`.

**Test scenarios:**
- Happy path: GET `/` lista tasks não-arquivadas, ordenadas por `created_at DESC`.
- Happy path: POST `/tasks` cria task e anexa via Turbo Stream na lista sem reload.
- Happy path: clicar Start numa task cria TimeEntry e UI mostra "running" com contador.
- Edge case: clicar Start numa task B enquanto A está ativo → UI reflete que A parou e B está ativo (duas turbo-stream updates).
- Happy path: clicar Stop numa task ativa fixa a duração exibida no total acumulado.
- Integration: verificar que o Stimulus timer não causa memory leak ao navegar entre páginas (disconnect limpa interval).

**Verification:**
- `bin/rails test:system` passa.
- Operar a UI manualmente: criar task, iniciar/parar timer, ver contador atualizar.

- [ ] **Unit 4: Integração Shortcut (read-only)**

**Goal:** Dado um Shortcut Story ID (ex.: `123` ou `SC-123`), buscar dados da story via API e usar para preencher uma Task.

**Requirements:** R2

**Dependencies:** Unit 1 (modelo `ShortcutStory`), Unit 3 (form para oferecer o campo)

**Files:**
- Create: `app/services/shortcut/story_fetcher.rb`
- Create: `app/services/shortcut/client.rb` (wrapper Net::HTTP ou Faraday fino)
- Modify: `app/controllers/tasks_controller.rb` (aceita `shortcut_id` em `params`; delega ao fetcher)
- Modify: `app/views/tasks/_form.html.erb` (campo opcional "Shortcut ID")
- Create: `.env.example` com `SHORTCUT_API_TOKEN=`
- Modify: `Gemfile` (gem `dotenv-rails` se decidido) ou usar `ENV` nativo
- Test: `test/services/shortcut/story_fetcher_test.rb` (com stubs HTTP via Minitest + WebMock ou `Net::HTTP` stub)

**Approach:**
- Input normalizado: aceitar `SC-123` → `123`, ou só `123`.
- `Shortcut::Client#get_story(id)`: HTTPS GET `https://api.app.shortcut.com/api/v3/stories/{id}` com header `Shortcut-Token: ENV['SHORTCUT_API_TOKEN']`. Timeout 5s. Raise `Shortcut::NotFound` em 404, `Shortcut::AuthError` em 401, `Shortcut::Error` em outros.
- `Shortcut::StoryFetcher#call(id)`: chama `Client`, cacheia em `ShortcutStory` (upsert por `external_id`), retorna o registro. Re-fetch se `fetched_at` < 1h atrás.
- Task criada com `shortcut_id` puxa title do Shortcut se o title do usuário estiver vazio.
- Flash message se Shortcut indisponível — Task ainda é criada com o title fornecido, só sem o cache.

**Execution note:** Stub HTTP a nível de Faraday/Net::HTTP nos testes do Client. O Fetcher usa stub do Client.

**Patterns to follow:**
- Padrão "service object" 37signals: PORO em `app/services/`, chamado como `Shortcut::StoryFetcher.new.call(id)` ou `.call(id)` de classe.
- Erro hierarquia: `Shortcut::Error < StandardError`, subclasses específicas.

**Test scenarios:**
- Happy path: `Client#get_story(123)` com 200 retorna hash parseado.
- Edge case: `get_story("SC-123")` → normaliza para 123 e chama endpoint correto.
- Error path: 404 levanta `Shortcut::NotFound`.
- Error path: 401 levanta `Shortcut::AuthError` com mensagem clara.
- Error path: timeout (Net::ReadTimeout) levanta `Shortcut::Error` com root cause.
- Happy path: `StoryFetcher#call` persiste ShortcutStory na primeira chamada.
- Happy path: segunda chamada para mesmo ID em <1h retorna do cache sem HTTP.
- Edge case: `fetched_at` stale (>1h) re-fetch e atualiza `ShortcutStory`.
- Integration: criar Task via UI com Shortcut ID preenche `shortcut_story` e copia title quando title do user está vazio.

**Verification:**
- Com `SHORTCUT_API_TOKEN` válido em `.env`, criar uma Task com `SC-<id_real>` popula title e associa ShortcutStory.
- Sem token, endpoint retorna erro legível e Task é criada só com o que o usuário digitou.

- [ ] **Unit 5: Agenda + relatórios**

**Goal:** Visão agrupada do que foi feito (por dia, com tasks e duração total por task) e relatórios agregados (totais por dia/semana/story).

**Requirements:** R4, R5

**Dependencies:** Unit 3 (dados de TimeEntry precisam estar populados para renderizar)

**Files:**
- Create: `app/controllers/agenda_controller.rb`, `app/controllers/reports_controller.rb`
- Create: `app/views/agenda/index.html.erb`, `app/views/reports/index.html.erb`
- Create: `app/models/concerns/time_entry_aggregations.rb` (scopes: `on_day(date)`, `grouped_by_day`, `total_duration`)
- Modify: `config/routes.rb` (`get :agenda`, `get :reports`)
- Test: `test/models/concerns/time_entry_aggregations_test.rb`, `test/controllers/agenda_controller_test.rb`

**Approach:**
- Agenda: timeline por dia (dia atual + 7 anteriores por padrão), cada dia listando tasks trabalhadas e duração somada. Navegação para semanas anteriores via link (`?week=2026-W16`).
- Relatórios (v1 mínimo): totais por dia (últimos 30 dias, gráfico simples), totais por Shortcut story (top 10), totais por epic (via `shortcut_story.epic_name` quando disponível).
- Cálculo de duração: `time_entries.sum("COALESCE(ended_at, CURRENT_TIMESTAMP) - started_at")` — entry ativa conta até agora.
- Sem biblioteca de charting no v1 — tabelas + barras renderizadas com Tailwind (div com `bg-blue-500` e `style="width: X%"` para barras proporcionais). Adicionar chart.js ou similar em follow-up se necessidade real aparecer.

**Patterns to follow:**
- Rails guides `active_record_querying` (grouping, sums).
- 37signals: models "fat", lógica de agregação fica em concerns do modelo, não nos controllers.

**Test scenarios:**
- Happy path: `TimeEntry.on_day(Date.today)` retorna apenas entries com `started_at` no dia.
- Edge case: entry que cruza meia-noite conta no dia do `started_at` (decisão explícita; documentar no teste).
- Happy path: `TimeEntry.grouped_by_day` retorna hash `{ date => [entries] }` ordenado desc.
- Edge case: entry ativa (ended_at nil) considera tempo até agora no cálculo de duração.
- Happy path: GET `/agenda` renderiza dias com entries agrupadas; dias sem atividade aparecem vazios até 7 dias atrás.
- Happy path: GET `/reports` mostra top tasks por tempo gasto.

**Verification:**
- Com fixtures cobrindo 3 dias e 2 tasks, a agenda mostra agrupamento correto e totais batem com soma manual.

- [ ] **Unit 6: Quick-access Rust (gtk-rs + tray)**

**Goal:** App nativo Linux com ícone na bandeja e mini-janela para ações rápidas (start por Shortcut ID, stop do ativo, listar ativos).

**Requirements:** R6

**Dependencies:** Unit 2 (API existe e está estável)

**Files:**
- Create: `quick-access/Cargo.toml`, `quick-access/src/main.rs`
- Create: `quick-access/src/api_client.rs` (reqwest blocking; structs com serde)
- Create: `quick-access/src/tray.rs` (ksni implementation — ícone, tooltip, menu "Stop active" / "Open main app")
- Create: `quick-access/src/ui/mini_window.rs` (janela gtk4 + libadwaita com entry field e submit button)
- Create: `quick-access/resources/icons/buckland-idle.svg`, `quick-access/resources/icons/buckland-running.svg`
- Test: `quick-access/tests/api_client_test.rs` (integração com mock server via `mockito` ou `httpmock`)

**Approach:**
- Dois ícones de tray: idle (sem timer) e running (com timer ativo). Tooltip mostra "nome da task — MM:SS".
- Menu do tray: "New timer...", "Stop active", "Open buckland", "Quit".
- "New timer..." abre mini window (pequena, decorada como dialog GNOME) com um campo "Shortcut ID or task title" e botão Start.
- `api_client.rs`: funções tipadas `start_timer(input)`, `stop_active()`, `get_active()`, `list_tasks()`. `input` aceita Shortcut ID (`SC-123`, `123`) ou texto livre — a API Rails decide o que fazer.
- Polling leve: thread separada chamando `get_active()` a cada 10s, atualiza estado do tray via channel (gtk main thread).
- Base URL em `~/.config/buckland/config.toml` (default `http://127.0.0.1:3000`).

**Execution note:** Começar com teste de integração do `api_client.rs` contra um mock HTTP server rodando os mesmos contratos que o Rails expõe.

**Patterns to follow:**
- gtk-rs book para main loop e signals.
- `ksni` crate examples para SNI tray icon.
- Error handling: `Result<T, BucklandError>` com `thiserror` para tipagem.

**Test scenarios:**
- Happy path: `api_client::start_timer("SC-123")` contra mock retorna `TimeEntry` parseado.
- Error path: mock retorna 401 → `BucklandError::Auth`.
- Error path: mock indisponível (conexão recusada) → `BucklandError::Unreachable`, tray mostra ícone de erro.
- Happy path: `get_active` sem ativo (204 ou null) → `None`, tray volta a idle.
- Integration: ciclo completo — start via mini window, confirmar via `get_active`, stop via menu tray, confirmar `None` via polling.

**Verification:**
- `cargo test` passa com mocks.
- Build `cargo build --release` produz binário em `quick-access/target/release/buckland-quick`.
- Rodar binário com Rails local em `:3000` mostra ícone no tray; start e stop funcionam end-to-end.

## System-Wide Impact

- **Interaction graph:** UI web e Rust tray ambos mutam o mesmo estado via camada de API. Qualquer comportamento implementado na UI web (ex.: parar ativo ao iniciar novo) precisa valer também na API — `TimerOperations` concentra isso.
- **Error propagation:** Shortcut API indisponível não deve quebrar criação de Task (degradação graciosa). Rails API indisponível deve mostrar estado de erro claro no tray, não crashar o app Rust.
- **State lifecycle risks:** Entry ativa órfã se o Rails crashar no meio do start — índice único parcial impede inconsistência duradoura, mas usuário pode precisar de ação manual "Stop all" em caso raro.
- **API surface parity:** UI web e tray Rust consomem o mesmo endpoint para `start_timer` — mantém semântica idêntica sem duplicar regra.
- **Integration coverage:** testes unitários do `TimerOperations` + testes de API (Unit 2) + teste de integração do api_client Rust (Unit 6) cobrem o fluxo completo.
- **Unchanged invariants:** Shortcut API é consumida somente via `Shortcut::Client`; nenhuma outra parte do código fala direto com `api.app.shortcut.com`.

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Shortcut API rate limit ou downtime | Cache em `ShortcutStory` com TTL de 1h; degradação graciosa quando falha. |
| Duas TimeEntry ativas por race condition | Índice único parcial no banco é a defesa final; transação em `TimerOperations#start!` previne no caminho normal. |
| Autor ainda iniciante em Rust | Unit 6 é a última — stack Rails funcionando sozinha entrega valor. Começar Rust com `api_client` (o mais simples) antes da UI. |
| Polling de 10s no tray consome bateria em notebook | Aceitável no v1; trocar por SSE/WebSocket se incomodar. Documentar no config file. |
| Drift entre contrato JSON do Rails e o que o client Rust espera | Testes do Client Rust reproduzem os mesmos payloads dos testes da API Rails. Mudanças de contrato requerem update nos dois lados. |

## Documentation / Operational Notes

- Criar `README.md` na raiz com: como rodar o Rails (`bin/dev`), como rodar o quick-access (`cargo run` em `quick-access/`), variáveis de ambiente necessárias.
- Documentar no `.env.example` todas as ENV vars (`SHORTCUT_API_TOKEN`).
- Considerar `docs/solutions/` conforme problemas forem resolvidos durante a execução (padrão superpowers/compound).

## Sources & References

- **Origin document:** [docs/claude/plan/buckland.md](../claude/plan/buckland.md)
- Shortcut API: https://developer.shortcut.com/api/rest/v3
- Rails guides: https://guides.rubyonrails.org/
- gtk-rs book: https://gtk-rs.org/gtk4-rs/stable/latest/book/
- ksni crate: https://crates.io/crates/ksni
