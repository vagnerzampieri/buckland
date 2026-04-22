# Buckland — Development Guidelines

## Project

Personal time-tracking app with Shortcut integration. Two components:

- **Rails 8+ app (main)** — source of truth. SQLite + Hotwire (Turbo + Stimulus) + Solid Queue + Minitest. Serves the full UI (todo-list, agenda, reports) and a JSON API on `127.0.0.1:3000/api/...`.
- **Rust `quick-access` companion (`quick-access/`)** — GTK4/libadwaita tray app (gtk-rs + ksni). Consumes the Rails API for start/stop/active. Linux-only. Its own conventions live alongside its crate.

Plan of record: [docs/plans/2026-04-22-001-feat-buckland-timetracking-plan.md](docs/plans/2026-04-22-001-feat-buckland-timetracking-plan.md).

### Shortcut Integration

Read-only. Given a story ID (`123` or `SC-123`), `Shortcut::Client` fetches from `api.app.shortcut.com`, `Shortcut::StoryFetcher` caches to `ShortcutStory` (TTL 1h). Credential in `ENV['SHORTCUT_API_TOKEN']`. No write-back, no status changes.

### Timer Invariant

At most one `TimeEntry` with `ended_at IS NULL` at any time. Enforced by a partial unique index and by `TimerOperations#start!` (stops any active entry inside the same transaction before starting a new one).

## Core Philosophy

Write code that is easy to change, easy to understand, and easy to delete. Favor simplicity over cleverness. Follow Ruby idioms and Rails conventions unless there is a concrete reason not to.

## TDD & Tidy First — Kent Beck

From *Test Driven Development: By Example* and *Tidy First?*. TDD is a design tool, not just verification:

1. **Red** — write a failing test that describes the behavior you want. The test shapes the API before the code exists.
2. **Green** — write the simplest code that makes the test pass. No more.
3. **Refactor** — clean up duplication and improve design while keeping tests green.

Rules:
- Never write production code without a failing test.
- Make each step as small as possible. If a step feels big, break it down.
- When stuck, write a simpler test.
- Tests are first-class code — keep them clean and readable.

**Separate structural changes from behavioral changes.** Tidy First says: if you want to add a feature but the code isn't ready, do the tidying as its own commit, then add the feature in a second commit. Never mix them. A reviewer (human or agent) should be able to tell at a glance whether a commit changes behavior or only reshuffles structure.

## Refactoring — Martin Fowler

From *Refactoring: Improving the Design of Existing Code* (2018 edition):

- Refactor in small, named steps (Extract Method, Inline Variable, Move Method, Introduce Parameter Object, Replace Conditional with Polymorphism, etc.).
- "Make the change easy, then make the easy change." Preparatory refactoring lands before the feature, not as part of it (see Tidy First above).
- Each refactoring step should keep tests passing. If tests break, the step was too big.
- Watch for code smells: Long Method, Large Class, Feature Envy, Data Clump, Primitive Obsession, Shotgun Surgery, Divergent Change.
- Refactoring preserves behavior. If you are changing behavior, it is not a refactor — it is a feature or a fix.

## Clean Code — Robert C. Martin

From *Clean Code*:

- **Names reveal intent.** A name should tell you why it exists, what it does, and how it is used. If a name requires a comment, the name is wrong.
- **Methods do one thing.** They should be small, do one thing, and do it well.
- **Single Responsibility Principle.** A class has one reason to change. A method has one level of abstraction.
- **No side effects.** A method named `fetch_story` should not also persist a `ShortcutStory`.
- **Don't Repeat Yourself** — but only extract when you see real duplication (three or more occurrences), not structural similarity.
- **Boy Scout Rule.** Leave the code cleaner than you found it — but only in code you are already touching.

## Object Design — Sandi Metz

From *Practical Object-Oriented Design in Ruby* (POODR) and *99 Bottles of OOP*.

Her four rules, unadapted:

1. **Classes can be no longer than 100 lines of code.**
2. **Methods can be no longer than 5 lines of code.**
3. **Methods can take no more than 4 parameters** — and a hash counts as parameters.
4. **Controllers can instantiate only one object.** One instance variable per controller action; views reach state through that object.

Principles (as important as the rules):

- **Inject dependencies; do not hardcode them.** A class that creates its own collaborators hides its dependency graph inside methods and is hard to test in isolation. Pass collaborators in through the initializer or as keyword arguments. The test doubles you need and the seams you want both become obvious.
- **Depend on behavior, not data.** Talk to collaborators through small, focused interfaces (in Ruby, duck-typed messages). Don't reach into objects for their internals.
- **Prefer duplication over the wrong abstraction.** Wait until the pattern is real before extracting. A premature abstraction is more expensive than three copies.
- **Favor composition over deep inheritance.** Flat, composable objects beat tall hierarchies.

Break a rule when you have a concrete reason. Then write the reason down.

## Rails Conventions First — David Bryant Copeland

From *Sustainable Web Development with Ruby on Rails*. Rails ships with strong, opinionated structure. Most apps spend their lifetime inside it and never need more:

- **Use what the framework already offers** — ActiveRecord, concerns, callbacks, helpers, partials, jobs, Current attributes — before introducing any new layer.
- **Fat models, thin controllers.** Business rules belong in models. Controllers parse, dispatch, and render. One instance variable per action (Metz rule 4).
- **Validations, constraints, and database enforcement together.** `NOT NULL`, foreign keys, unique indexes, partial indexes at the DB level. Model validations reinforce them. Never trust one layer alone.
- **Lean on conventions so that a new reader knows where to look.** `app/models/task.rb` defines a `Task`. `tasks_controller.rb` handles `/tasks`. If you break this, you pay compounding interest.
- **Callbacks and concerns are the first tools for sharing behavior.** Reach for them before inventing a service framework.
- **Minitest + fixtures is enough.** Rails ships it; it runs fast; it reads cleanly. Only introduce RSpec/FactoryBot if there is a concrete reason the team agrees on.

The posture: **do not add a layer until the existing ones hurt.** Pain has to be specific and named — "this callback has grown to 40 lines and couples three models", not "I've heard services are cleaner".

## Layered Design When Conventions Aren't Enough — Vladimir Dementyev

From *Layered Design for Ruby on Rails Applications*. When an app outgrows Copeland's baseline, add layers on purpose — not by reflex. Each layer earns its existence by solving a specific problem:

- **Service objects** (`app/services/`) — coordinate operations that span models or cross an external boundary. Example in buckland: `Shortcut::StoryFetcher` — it calls the Shortcut API, caches into `ShortcutStory`, and returns a record. One public method (`#call`). No base class, no registry, no DSL.
- **Form objects** — use when input spans multiple models or needs validations that don't belong on a persisted record. Skip until you have two of them; one is not a pattern.
- **Policy objects** — use when authorization rules grow beyond `before_action` guards. Buckland is single-user, so none needed yet.
- **Presenters / decorators** — use when views accumulate formatting logic a helper can't cleanly express. Partials and helpers come first.
- **Query objects** — use when a scope chain becomes its own domain concept or needs tests in isolation.

Dementyev's key point: **each layer needs a reason to exist, not just a folder**. If a "service" is a thin wrapper over one `ActiveRecord` call, delete it and inline it. Layering is architectural intent made visible, not organizational theater.

## Principles as Filters, Not Dogma

SOLID, KISS, YAGNI work best as questions, not commandments:

- **SRP** — *Does this class have more than one reason to change?* If yes, consider splitting.
- **OCP** — *Can I extend this behavior without editing this class?* If you need polymorphism, put the seam where change is expected.
- **LSP** — *Can subclasses substitute the parent without surprising callers?* If not, you probably want composition instead of inheritance.
- **ISP** — *Am I forcing a client to depend on methods it doesn't use?* Split interfaces (in Ruby, split duck-typed messages).
- **DIP** — *Is this depending on a concrete class when a collaborator would do?* Inject.
- **KISS** — *Is there a simpler design that still solves the whole problem?*
- **YAGNI** — *Do I have a concrete, current requirement for this flexibility? Or am I speculating?* If speculation, cut it.

Ask the question. If the answer is "no, it's fine", move on without the ceremony.

## Hotwire & Frontend

Concrete conventions for buckland's UI:

- **Hotwire first.** Turbo Drive for navigation, Turbo Frames for scoped updates, Turbo Streams for server-pushed changes, Stimulus for client-only behavior. Reach for a JS framework only when Hotwire genuinely can't express the interaction.
- **Views render state; helpers format it.** Logic in views means a helper or partial is missing. Partials are the unit of reuse.
- **Stimulus controllers do one thing.** Clean up in `disconnect()` — intervals, listeners, observers. A Stimulus controller is the DOM analog of a Sandi-Metz-sized Ruby class: small, focused, composable.
- **The API namespace (`app/controllers/api/`) is a contract** with the Rust quick-access. Breaking changes require updating both sides in the same commit.

## Testing Approach

Tests are the first consumer of your design — if they are painful to write, the design is wrong:

- **Minitest + fixtures** by default. Matches Rails 8 out-of-the-box.
- **Model tests** cover domain logic in isolation: validations, scopes, concern behavior, invariants (e.g., single active timer).
- **Controller tests** cover routing, params, authorization, and response shape.
- **System tests** (Capybara + headless Chrome) cover end-to-end user flows: creating a task, starting a timer, viewing the agenda.
- **Service tests** stub at the HTTP boundary — WebMock or `Net::HTTP` stubs for Shortcut. Don't stub your own classes.
- Test behavior, not implementation. A test that breaks when you rename a private method is testing the wrong thing.
- Each test is independent and repeatable.
- **Don't mock what you own.** If you feel the urge to mock a model or a service, either the test or the design has a problem — usually a hidden dependency that wants to be injected (see Metz).

## Sustainable Development

- **Follow conventions.** The best Rails code looks like the Rails guides. When in doubt, read the guide.
- **Boring is good.** Avoid clever metaprogramming, exotic gems, or DSLs that replace plain Ruby.
- **Database constraints matter.** Schema enforces what the code expects.
- **Migrations are permanent code.** Safe and reversible. Never edit a migration that has been applied in a shared environment.
- **Keep dependencies minimal.** Every gem is a liability.
- **Don't optimize for hypothetical scale.** Solve the problem in front of you.
- **Cost of change guides decisions.** Easy-to-change decisions can be deferred. Hard-to-change ones (schema, API contract with the Rust client, Shortcut integration shape) deserve more thought upfront.

## Code Style

### Ruby / Rails
- Follow `rubocop-rails-omakase` (Rails 8 default) or `standard` — pick one per repo and run it before every commit.
- Ruby 3.3+. Prefer pattern matching, endless methods, and `Data.define` when they fit.
- `snake_case` for methods and variables, `CamelCase` for classes and modules, `SCREAMING_SNAKE_CASE` for constants.
- No comments that restate the code. Comments explain *why*, never *what*.
- Prefer guard clauses and early returns over deep nesting.
- Use keyword arguments for any method with more than one argument.

### Views, Tailwind & Stimulus
- ERB. No ViewComponent until a real need emerges — partials first.
- **Tailwind CSS** for styling. Utility classes in the markup; extract with `@apply` in `app/assets/stylesheets/application.tailwind.css` only for genuinely repeated clusters (`.btn-primary`, `.card`), not cosmetic aliases.
- Dark mode via `dark:` variants + `prefers-color-scheme`. No theme toggle in v1.
- Stimulus controllers one per behavior, co-located under `app/javascript/controllers/`. `camelCase` identifiers in JS, `kebab-case` in HTML attributes.
- Prefer Turbo Streams over direct DOM manipulation in Stimulus.

## Commands

```bash
# Development
bin/dev                       # Rails + asset watcher (Foreman via Procfile.dev)
bin/rails server              # Rails only
bin/rails console             # REPL

# Database
bin/rails db:migrate          # Apply pending migrations
bin/rails db:rollback         # Undo last migration
bin/rails db:seed             # Load seed data
bin/rails db:reset            # Drop, recreate, migrate, seed

# Testing
bin/rails test                # All minitest unit + integration tests
bin/rails test:system         # System (Capybara) tests
bin/rails test test/models/task_test.rb  # Single file

# Quality
bundle exec rubocop           # Lint Ruby (if adopted)
bin/brakeman                  # Security audit
bin/rails routes              # Inspect routes

# Rust quick-access (in quick-access/)
cargo run                     # Dev run
cargo test                    # Run tests
cargo fmt && cargo clippy     # Format + lint
cargo build --release         # Release binary
```
