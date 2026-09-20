# Engineering progress log

> Internal, append-only handoff log. Put mutable task state in
> [trackers.md](trackers.md); keep this file as concise evidence of what changed
> and how it was checked.

## Current snapshot

- Current focus: `ARCH-004` cloud/local analysis router
- Current branch: `dev`
- Baseline commit: `78245b47a`
- Baseline working tree: clean before the project-memory files were added
- Repository map: [features.md](features.md)
- Work, risks, and decisions: [trackers.md](trackers.md)

## 2026-09-20 — LM Studio GGUF activation bridge

### Outcome

- Added `codex local-model activate-lm-studio <model-id> [--dry-run]`.
- Activation requires a registered, present GGUF artifact and configured
  loopback analysis URL/model.
- The bridge preserves the Codex artifact with `lms import --copy`, requests
  full GPU offload, assigns the configured backend model identifier, binds the
  API server to `127.0.0.1`, and verifies availability after startup.
- Dry-run mode prints quoted commands without importing, loading, or starting
  the server.

### Validation

- `cargo check -p codex-cli --lib` — passed after the activation bridge.
- `cargo fmt --all` — completed with the repository's existing stable-Rust
  warnings for nightly-only import formatting.

### Decisions, risks, and follow-up

- LM Studio integration is explicit and backend-specific; the generic analysis
  adapter remains OpenAI-compatible and backend-neutral.
- Live activation still requires a downloaded registered GGUF and a functioning
  LM Studio desktop daemon. Use `--dry-run` first on a new setup.

## 2026-09-20 — Local-analysis backend status

### Outcome

- Added `codex local-model status [--json]` to probe the configured loopback
  OpenAI-compatible `/models` endpoint with a five-second timeout.
- The command reports the exposed model IDs and fails clearly when the
  configured `backend_model` is absent.
- Reused the same HTTP(S)-loopback validation as inference so the status path
  cannot probe arbitrary remote hosts.
- Added a strict OpenAI-compatible `json_schema` response request for evidence
  digests while retaining Codex-side schema, identity, confidence, and citation
  validation.

### Validation

- `cargo test -p codex-local-models` — 27 tests passed, including mocked model
  discovery/matching, structured-output request verification, and rejection of
  a non-loopback status endpoint.
- `cargo check -p codex-cli --lib` — passed.
- `cargo fmt --all` — completed; stable Rust emitted the repository's existing
  nightly-only `imports_granularity` warnings.

### Decisions, risks, and follow-up

- The mocked integration verifies the endpoint contract without requiring a
  particular local-serving product.
- `ARCH-004` remains in validation until `local-model status` and an eligible
  completed command are exercised against an actual configured local server.
- Machine inspection found an NVIDIA RTX 3090 with 24 GiB VRAM and LM Studio
  CLI 1.3.3, but no downloaded LM Studio model. The CLI could not wake its
  desktop daemon and no API listener appeared on ports 1234 or 11434.

## 2026-09-20 — Interactive completion routing and telemetry

### Outcome

- Connected commands that finish through `write_stdin` to the same opt-in,
  fail-open local-analysis pipeline used by immediately completed commands.
- Added routing counters and artifact-size histograms with stable outcome and
  bypass labels only.
- Excluded command text, raw output, artifact paths, findings, and local-model
  responses from routing telemetry.

### Validation

- `cargo check -p codex-core --lib` with two build jobs — passed after the
  interactive-completion and telemetry integration.
- Existing 25-test local-model suite remains the behavioral coverage for
  classification, routing, adapter validation, and fallback.

### Decisions, risks, and follow-up

- Interactive routing analyzes the output segment returned at completion; prior
  streamed chunks remain in the conversation and are not silently discarded.
- `ARCH-004` moves to validation. Next add a backend health/status command and
  exercise a live loopback server before calling the router complete.

## 2026-09-20 — Completed-command local-analysis routing

### Outcome

- Added deterministic classification for common test, lint, scanner, build,
  and log commands.
- Added one fail-open orchestration function covering capture, policy routing,
  bounded reads, loopback inference, and evidence validation.
- Connected completed `exec_command` output to the opt-in router. Successful
  analysis returns a compact untrusted digest and raw-artifact reference to the
  cloud model; every bypass or failure leaves the original output unchanged.
- Added a 30-second inference timeout and retained the selected cloud model as
  the only component allowed to diagnose, edit, or accept results.

### Validation

- `just test -p codex-local-models` with two build jobs — 25 tests passed,
  including command classification, end-to-end evidence routing, and HTTP-error
  fallback.
- `cargo check -p codex-core --lib` with two build jobs — passed after the
  completed-command hook and explicit core HTTP dependency were added.

### Decisions, risks, and follow-up

- Local analysis currently covers completed `exec_command` responses. Output
  completed later through `write_stdin` still follows the existing raw path.
- Local digests are labeled untrusted and include the preserved artifact path
  and byte count so the cloud model can request or inspect raw evidence.
- Next connect interactive completion and emit routing/bypass telemetry without
  logging raw output or secrets.

## 2026-09-20 — Durable evidence capture and loopback analysis adapter

### Outcome

- Added content-addressed raw-output capture with idempotent SHA-256 paths and
  bounded reads for local-model input; the complete artifact remains available
  for citation checks and cloud fallback.
- Added a read-only OpenAI-compatible chat-completions adapter that treats raw
  output as untrusted data, requests the versioned evidence schema, and rejects
  invalid response JSON or citations.
- Restricted analysis endpoints to HTTP(S) loopback hosts, preventing this path
  from sending repository logs to an arbitrary remote server.
- Added `backend_url` and `backend_model` settings; routing bypasses local
  analysis when serving configuration is absent.

### Validation

- `just test -p codex-local-models` with two build jobs — 22 tests passed,
  including durable capture, bounded input, valid adapter output, and remote
  endpoint rejection.
- `just write-config-schema` — passed and generated backend configuration.
- `cargo check -p codex-core --lib` with two build jobs — passed.

### Decisions, risks, and follow-up

- Downloaded artifacts are not executed automatically. A user-controlled local
  server must expose the configured model on a loopback OpenAI-compatible API.
- Raw evidence remains authoritative; the local digest is untrusted and cannot
  request tools, edits, or commands.
- Next classify completed commands and connect this opt-in path to captured
  aggregated output, with failure falling back to the existing cloud/raw path.

## 2026-09-20 — Local-analysis policy and evidence envelope

### Outcome

- Added opt-in `[local_models.analysis]` configuration with a registered model
  ID, minimum routing threshold, and maximum local-model input size.
- Added deterministic routing decisions for test, lint, scanner, log, and build
  artifacts. Disabled, undersized, unregistered, invalid-policy, or
  raw-unavailable cases bypass local analysis and preserve cloud/raw handling.
- Added SHA-256 artifact identities and a versioned local evidence digest with
  byte-range citations, confidence bounds, and explicit unknowns.
- Wired the effective analysis policy into core configuration while keeping it
  disabled by default and keeping the selected cloud model authoritative.

### Validation

- `just test -p codex-local-models` with two build jobs — 19 tests passed,
  including routing, raw-fallback, valid citations, and out-of-bounds rejection.
- `just write-config-schema` — passed and generated the analysis settings.
- `cargo check -p codex-core --lib` with two build jobs — passed.

### Decisions, risks, and follow-up

- Local output is untrusted evidence, never an instruction or autonomous patch.
- A raw artifact must remain available whenever local analysis is selected so
  the cloud model can inspect source evidence or bypass a weak summary.
- Next integrate raw artifact capture and a read-only local inference adapter;
  no command output is automatically routed yet.

## 2026-09-20 — Hugging Face inspection and revision resolution

### Outcome

- Added `codex local-model inspect <owner/repository>` with human-readable and
  JSON output for resolved SHA, file metadata, pipeline, gated/private, and
  disabled state.
- Downloads now accept a branch, tag, or commit, resolve it through model-info,
  validate the returned full SHA, and retain the immutable SHA in storage and
  registry paths.
- Continued to source private/gated authentication only from `HF_TOKEN`.

### Validation

- `just test -p codex-local-models` with two build jobs — 15 tests passed,
  including successful model inspection and invalid returned-SHA rejection.
- `cargo build -p codex-cli --bin codex` with two build jobs — passed.
- Live `local-model inspect google-bert/bert-base-uncased --revision main` —
  resolved commit `86b5e0934494bd15c9632b12f734a8a67f723594` and reported 16 files.

### Decisions, risks, and follow-up

- Resolution is a read-only metadata call; artifact download remains pinned to
  the returned immutable commit to avoid branch/tag drift during transfer.
- `ARCH-003` is complete for the scoped single-artifact lifecycle.
- `ARCH-004` is now active: define the evidence envelope and routing policy for
  bounded local analysis under a cloud-owned problem-solving loop.

## 2026-09-20 — Local-model quota enforcement and safe removal

### Outcome

- Enforced `[local_models.storage].max_disk_gb` before and during artifact
  streaming, accounting for existing model and staging storage.
- Added `codex local-model remove <model-id>` with registry lookup and a strict
  containment check that refuses to delete paths outside the configured model
  directory.
- Missing artifact files can be cleaned from the registry, while unknown model
  IDs return an explicit command error.

### Validation

- `cargo fmt --all` — passed with the existing stable-rustfmt warnings about
  the nightly-only import-granularity setting.
- `just test -p codex-local-models` with two build jobs — 13 tests passed,
  including quota rejection, safe artifact removal, and outside-root refusal.
- `cargo build -p codex-cli --bin codex` with two build jobs — passed.
- Smoke-tested `local-model remove --help` and the explicit error returned for
  an unknown model ID against an isolated, nonexistent model directory.

### Decisions, risks, and follow-up

- A zero-GiB configured quota intentionally rejects every non-empty download.
- Removal does not recursively delete directories and never follows a registry
  path outside `models_dir`.
- `ARCH-003` remains active for revision resolution and remote inspection.

## 2026-09-19 — Versioned local-model registry and CLI inspection

### Outcome

- Added a schema-versioned registry with immutable Hugging Face source
  identity, artifact format, quantization, and absolute local path metadata.
- Added atomic normalized registry persistence, missing-file defaults, upsert,
  removal primitives, and deterministic model ordering.
- Added usable `codex local-model path` and `codex local-model list [--json]`
  commands. They inspect storage and registry state only; they do not change the
  selected cloud model or imply that local inference is active.
- Moved the workspace handoff path from `D:\AI\Coding\codex` to
  `E:\AI\Coding\codex` after the user relocated the repository.

### Validation

- `cargo fmt --all` — passed; stable rustfmt emitted existing warnings about a
  nightly-only import-granularity setting.
- `just test -p codex-local-models` with two build jobs — 9 tests passed.
- `just test -p codex-cli` with two build jobs — compiled the complete CLI/core
  graph and ran 446 tests: 445 passed and one unrelated existing Windows doctor
  snapshot failed because a temporary config path was not normalized.
- Smoke-tested `local-model path`, empty `local-model list`, JSON list output,
  and `CODEX_LOCAL_MODELS_DIR=E:\AI\Models\codex` against an isolated Codex
  home.
- Refreshed the Bazel module dependency lock successfully; no lockfile content
  change was required for the new workspace-local dependencies.

### Decisions, risks, and follow-up

- `ARCH-002` is complete; constrained compilation resolves `RISK-006`.
- `ARCH-003` is active. Next implement pinned Hugging Face downloads that
  promote verified artifacts atomically and update this registry.
- `RISK-007` records the unrelated Windows doctor snapshot failure; its
  generated `.snap.new` file was removed rather than accepted.

## 2026-09-20 — Pinned Hugging Face artifact download

### Outcome

- Added `codex local-model download` for a single explicitly named artifact.
- Requires a full immutable 40-character commit hash, validates repository,
  model ID, filename, and optional SHA-256 inputs before network access.
- Supports ranged resume from `.partial` staging files, optional `HF_TOKEN`
  authentication, streaming writes, SHA-256 verification, atomic promotion,
  and registry upsert.
- Keeps cloud-model selection unchanged; downloading an artifact does not
  activate it or delegate reasoning work.

### Validation

- `just test -p codex-local-models` with two build jobs — 10 tests passed,
  including a mock HTTP download, checksum verification, artifact promotion,
  and registry update.
- `cargo check -p codex-cli --bin codex` with two build jobs — passed across
  the complete CLI dependency graph in the relocated workspace.
- `cargo build -p codex-cli --bin codex` with two build jobs — passed and
  produced an updated Windows debug executable.
- Smoke-tested the built executable's `local-model download --help` and
  `local-model path` surfaces — both exited successfully and reported the
  expected arguments and resolved storage paths.
- `bazel mod deps --lockfile_mode=update` — passed; the existing Bazel module
  lock required no content change.
- Internal architecture, feature, tracker, and progress Markdown — formatted
  with the repository's configured Prettier version.

### Decisions, risks, and follow-up

- The first download contract deliberately handles one file, making GGUF the
  practical first format while avoiding unsafe guesses about repository files.
- Branch/tag resolution is not accepted yet: callers must pin the full commit.
- `HF_TOKEN` is read only from the environment and is never persisted in the
  registry or accepted in shell history as an argument.

## 2026-09-19 — Local-model storage foundation

### Outcome

- Added the focused `codex-local-models` crate with typed storage settings and
  deterministic resolution for model, temporary-download, and registry paths.
- Wired `[local_models.storage]` into `ConfigToml`, the generated configuration
  schema, and Codex's effective runtime configuration.
- Defined model-root precedence as command override, then
  `CODEX_LOCAL_MODELS_DIR`, then `config.toml`, then `<CODEX_HOME>/models`.
- Clarified that the selected cloud model remains the authoritative brain;
  local models are planned for bounded test/log/scanner analysis with cited
  evidence and raw-artifact fallback.

### Files and surfaces

- `codex-rs/local-models/` — storage types, resolver, tests, and Bazel target.
- `codex-rs/config/src/config_toml.rs` — user configuration surface.
- `codex-rs/core/src/config/mod.rs` — effective runtime configuration.
- `codex-rs/core/config.schema.json` — generated schema.
- Workspace manifests and lockfiles — new crate registration and dependency.
- `docs/internal/architecture.md` — configuration, routing boundary, and
  planned user workflow.

### Validation

- `just fmt` — passed; unrelated Windows line-ending churn from Buildifier was
  reverted while retaining the intended formatted files.
- `just test -p codex-local-models` — 5 tests passed.
- `just write-config-schema` — passed and generated the local-model fields.
- `just test -p codex-config` — 340 tests passed.
- `bazel mod deps --lockfile_mode=update` through Bazelisk 1.29.0 / Bazel 9 —
  passed; the existing Bazel module lock required no content change.
- `just test -p codex-core` — build did not complete because concurrent Rust
  compilation exhausted Windows memory/page-file capacity (OS error 1455); no
  test assertion failed. Tracked as `RISK-006`.

### Decisions, risks, and follow-up

- `ARCH-002` is in validation pending a constrained-concurrency core test.
- `ARCH-004` now explicitly covers cloud-owned reasoning with local evidence
  analysis, not autonomous local implementation.
- The current executable does not yet download, serve, select, or route work to
  Hugging Face models; those remain `ARCH-003` and `ARCH-004`.

## 2026-09-19 — Hugging Face local-model architecture

### Outcome

- Extended `architecture.md` so users can discover, inspect, download,
  register, activate, serve, switch, and remove compatible Hugging Face models.
- Added revision pinning, resumable downloads, integrity metadata, gated-model
  authentication, license acknowledgement, secret handling, disk quotas, and
  custom-code safeguards.
- Added a versioned local model registry, backend adapters, capability-based
  selection, VRAM preflight, health checks, and safe runtime switching.
- Refined the hybrid loop around structured patches, isolated worktrees,
  failure classification, bounded repair, durable state, and reproducible run
  metadata.

### Files and surfaces

- `docs/internal/architecture.md` — updated system design and delivery phases.
- `docs/internal/trackers.md` — recorded `ARCH-001` and `DEC-003`.

### Validation

- Reviewed the design against existing model-provider, Ollama, LM Studio,
  app-server, agent, worktree, sandbox, and secret-storage capabilities.
- Markdown formatting checked with the repository's configured Prettier
  version.

### Decisions, risks, and follow-up

- Begin with one artifact/backend path before implementing a full format and
  runtime compatibility matrix.
- Cross-provider planner/executor routing and the exact public CLI/app-server
  API remain implementation-design work.

## 2026-09-19 — Repository baseline and project memory

### Outcome

- Surveyed the root layout, Rust workspace, CLI entry point, core runtime,
  current TUI/app-server architecture, protocol crates, extension families,
  SDKs, packaging, tests, and CI workflows.
- Recorded the principal product surfaces and runtime flow in
  [features.md](features.md).
- Counted the feature registry by lifecycle stage and documented that the Rust
  registry remains authoritative.
- Initialized active-work, backlog, risk, decision, and completed-work sections
  in [trackers.md](trackers.md).

### Evidence inspected

- `README.md`, `docs/install.md`, and `docs/contributing.md`
- `AGENTS.md` and the root `justfile`
- `codex-rs/Cargo.toml` and 154 package manifests under `codex-rs/`
- `codex-rs/cli/src/main.rs`
- `codex-rs/core/src/lib.rs`
- `codex-rs/tui/` and `codex-rs/app-server/`
- `codex-rs/protocol/` and `codex-rs/app-server-protocol/`
- `codex-rs/features/src/lib.rs`
- `sdk/typescript/`, `sdk/python/`, and `codex-cli/`
- `.github/workflows/`

### Validation

- Confirmed the starting Git branch and commit and verified that the worktree
  was clean before these files were created.
- The shared checkout moved from `main` to `dev` during inspection; both refs
  pointed to the same baseline commit, so the inspected source did not change.
- Cross-checked CLI capabilities against the `Subcommand` enum.
- Cross-checked feature lifecycle counts against `FeatureSpec` entries.
- Reviewed Markdown structure and relative links.
- No product code changed, so no product tests were required.
- Rust commands were not run because `rustc`, `cargo`, and `just` are not
  installed in the current environment; this is tracked as `RISK-001`.

### Next handoff

When a concrete change is selected:

1. Refresh the working branch, baseline commit, and relevant source anchors.
2. Move `NEXT-001` out of blocked state or replace it with a scoped work item.
3. Identify compatibility surfaces and required tests before editing.
4. Implement and validate according to `AGENTS.md`.
5. Update the tracker, append a dated entry here, and update the feature map if
   architecture or capabilities changed.

## Entry template

```markdown
## YYYY-MM-DD — <work item and short title>

### Outcome

- <observable change>

### Files and surfaces

- `<path>` — <reason>

### Validation

- `<command>` — <result>

### Decisions, risks, and follow-up

- <tracker IDs and next action>
```
