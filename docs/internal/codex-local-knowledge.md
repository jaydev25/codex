# Codex Local Engineering Knowledge

> Canonical handoff for improving the `codex-local` variant in this checkout.
> Detailed task state remains in [trackers.md](trackers.md), dated evidence in
> [progress.md](progress.md), and the full design in
> [architecture.md](architecture.md).

Actor reliability failures are recorded separately in the append-only
[local actor failure ledger](local-actor-failures.md).

## Purpose and product boundary

`codex-local` augments Codex with local inference while keeping the selected
cloud Codex model responsible for architecture, production implementation,
baseline tests, safety decisions, repair ownership, and final review. The local
model is an untrusted bounded tester. It may analyze large command output,
propose additional tests and focused commands, coordinate explicit monitor
phase gates, and attempt one evidence-backed repair, but it never receives
ambient filesystem or shell authority and never approves its own result.

The initial hardware target is one NVIDIA RTX 3090 with 24 GiB VRAM. The
validated local actor is Qwen 30B through LM Studio at a 65,536-token context.
A 131,072-token context was rejected because its estimated 24.47 GiB footprint
left no safe operating headroom.

## Source map

| Capability | Primary source |
| --- | --- |
| Local-model storage, registry, download, analysis, and usage ledgers | `codex-rs/local-models/` |
| Cloud-to-local actor request and response contract | `codex-rs/local-models/src/actor.rs` |
| Core `local_actor` tool, retry state, proposal validation, and execution-plan handoff | `codex-rs/core/src/tools/handlers/local_actor.rs` |
| Trusted structured-edit validation and patch rendering | `codex-rs/core/src/tools/handlers/local_actor_edits.rs` |
| Cloud developer/local tester guidance injected into model context | `codex-rs/core/src/context/local_actor_tester.rs` |
| Actor integration and resume tests | `codex-rs/core/tests/suite/local_actor.rs` |
| TUI session usage baseline, footer HUD, and `/status` reporting | `codex-rs/tui/src/chatwidget/` and `codex-rs/tui/src/status/` |
| Local development installer | `scripts/install/install-local-dev.ps1` |
| User setup for local inference | `docs/local-model-setup.md` |

## Implemented capability chain

1. Local-model storage resolves from a command override, then
   `CODEX_LOCAL_MODELS_DIR`, config, and finally `<CODEX_HOME>/models`.
2. Hugging Face inspection and downloads resolve immutable revisions, support
   resumable staging, enforce quotas, optionally verify SHA-256, and register
   artifacts without enabling repository-supplied code.
3. LM Studio activation imports a registered GGUF artifact, requests maximum
   GPU offload, starts a loopback server when needed, and verifies the selected
   API model.
4. Large deterministic command output can be stored locally and summarized by
   a loopback-only OpenAI-compatible analyst. Invalid analysis always falls
   back to the original raw evidence.
5. The cloud developer writes production code and baseline tests, then can
   invoke the strict `local_actor` function with a bounded test/review
   assignment containing an objective, allowed paths and tools, operation
   mappings, and acceptance criteria.
6. The actor receives that assignment in its system message and bounded source
   context in a separate lower-trust user message. It proposes additional
   tests, exact commands, diagnostics, or one narrow evidence-backed repair.
   Trusted Codex code checks context hashes and unique matches, renders any
   proposal, and returns the plan for cloud review and permission-aware
   execution.
7. Completed failures are reconstructed from rollout history. One failed test
   may receive one bounded actor repair attempt; a failed repair returns the
   original assignment and bounded evidence to the cloud developer. Unsafe
   operations and transport failures escalate immediately.
8. Explicit monitor assignments let the actor evaluate trusted polling output
   and coordinate already-authorized build, test, install, or restart phases.
   Development follow-ups return to the cloud developer.
9. The Windows development installer builds and hash-verifies `codex-local.exe`
   and `codex-code-mode-host.exe`, handles the required V8 artifact, and updates
   user PATH idempotently.

## Trust and safety invariants

- Local endpoints must be HTTP(S) loopback addresses.
- Tester assignments and actor results are strict, bounded schemas.
- Actor edits may touch only assignment-authorized paths. Replacement edits require
  complete source context, the current context hash, and old text that occurs
  exactly once. Trusted core code renders and reparses the final patch.
- Actor-proposed commands are data until the cloud reviews and invokes the
  normal execution tool.
- Raw command output, prompts, assignments, patches, paths, and model responses
  must not enter usage telemetry.
- Missing telemetry must not turn valid work into a product failure.
- Context remains incremental, bounded, and cache-conscious; no local feature
  may rewrite history or inject unbounded evidence.

## Usage and savings semantics

There are two intentionally different measurements:

- **Local analysis savings:** the persistent analysis ledger records raw and
  forwarded byte/token estimates. `estimated_cloud_input_tokens_avoided` is
  the saturating difference between raw and forwarded estimates. This is the
  only value shown as `Session tokens saved`.
- **Local actor usage:** successful actor completions record backend-reported
  prompt, completion, and total token counters in a separate ledger. The TUI
  reports calls and local tokens separately. These tokens are not added to the
  savings number because local and cloud tokenizers, prompts, cached context,
  reasoning, and output costs are not equivalent.

The TUI reads persistent totals at startup and subtracts that baseline when it
renders the footer or `/status`. Values are therefore current-process session
figures rather than misleading lifetime totals.

Do not introduce an actor “tokens saved” conversion without a reviewed
comparison model based on equivalent cloud work. Backend token usage alone is
evidence of local work performed, not evidence of cloud input avoided.

## Local actor reliability lesson

During `ARCH-011`, Qwen repeatedly returned semantically useful proposals with
invalid `apply_patch` envelopes: comments before a hunk, missing
`*** Begin Patch`, content without an `*** Update File` header, or an
unauthorized test command. The validator correctly rejected every proposal and
no actor-authored file change was applied.

Append each future failure to
[local-actor-failures.md](local-actor-failures.md) before changing retry or
validation behavior. The ledger intentionally stores classifications and
bounded reasons rather than model responses or repository content.

`ARCH-013` removed raw patch grammar from actor output, added bounded hashed
source context, and made safe edit-validation failures retryable. Continue to
improve the protocol boundary rather than weakening it:

- add deterministic repair or normalization only for harmless outer framing;
- keep paths and hunks reparsed by the trusted core;
- return precise structured validation errors to the next actor attempt;
- measure patch-format failure categories and model/version success rates;
- extend structured edits only when the new operation can retain equivalent
  trusted validation and bounded context;
- never execute a proposal merely because its intent is obvious.

## Development and validation workflow

Follow the root `AGENTS.md`. For Rust changes:

1. Keep work scoped and preserve unrelated dirty-tree edits.
2. Prefer focused modules; avoid growing central orchestration files.
3. Add integration coverage for agent behavior and snapshots for user-visible
   TUI changes.
4. Run `just fmt` in `codex-rs` after all code edits.
5. Run `just test -p <changed-crate>`; do not invoke `cargo test` directly.
6. For a large change, run `just fix -p <changed-crate>` after tests and do not
   rerun tests after the final `fix` or `fmt`.
7. Ask before running the complete workspace test suite when common, core, or
   protocol changes require it.
8. If dependencies change, run `just bazel-lock-update`; if config shapes
   change, regenerate the config schema.

Current focused checks for this feature family are:

```text
just test -p codex-local-models
just test -p codex-core local_actor
just test -p codex-tui status_snapshot_shows_session_local_token_savings
```

For intentional TUI changes, inspect `*.snap.new` before accepting the crate's
pending snapshots.

## Environment and known risks

- Rust, MSVC build tools, `just`, `cargo-nextest`, Bazelisk, and PowerShell 7
  are expected in the Visual Studio developer shell. `uv` and `dotslash` may
  still be absent from PATH.
- The machine now has 48 GiB system RAM; the former Cargo concurrency memory
  failure is resolved, but GPU context limits remain.
- A pre-existing Windows doctor snapshot has a machine-specific path issue.
- The normal Windows workspace sandbox currently fails to launch PowerShell
  with `CreateProcessWithLogonW failed: 2` and needs separate diagnosis.
- The branch is `local`, based on the documented baseline `78245b47a`, with a
  large user-owned working tree containing the codex-local feature series.

## How to maintain this file

Update this document only when the architectural boundary, source map,
operating procedure, or a durable engineering lesson changes. Put mutable task
status in `trackers.md`, append validation evidence to `progress.md`, and avoid
copying exhaustive flags or generated API shapes here.
