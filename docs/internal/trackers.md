# Engineering trackers

> Internal working state for repository changes. Keep entries concrete,
> source-linked, and small enough to hand off.

## Status vocabulary

| Status      | Meaning                                                |
| ----------- | ------------------------------------------------------ |
| Backlog     | Worth considering, but not selected for implementation |
| Ready       | Scoped and unblocked                                   |
| In progress | Actively being changed                                 |
| Blocked     | Cannot proceed without a named decision or dependency  |
| Validate    | Implemented; required checks or review remain          |
| Done        | Implemented and validated to the stated level          |
| Dropped     | Intentionally not proceeding; rationale recorded       |

## Active work

| ID       | Status      | Area                        | Outcome                                                                                                                                           | Owner | Next action                                                            |
| -------- | ----------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ----- | ---------------------------------------------------------------------- |

## Backlog

The backlog is intentionally empty. Do not infer a roadmap from feature flags,
TODO comments, or crate names. Add only work explicitly requested or agreed.

| ID  | Priority | Area | Desired outcome | Evidence / issue | Dependencies |
| --- | -------- | ---- | --------------- | ---------------- | ------------ |

## Completed

| ID       | Completed  | Area                   | Outcome                                                                                                                                 | Validation                                              |
| -------- | ---------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| ARCH-022 | 2026-09-29 | Tester-only orchestration contract | Removed the implementation task kind and planner-era runtime naming; cloud remains the only production developer while the local actor authors additional tests, reviews evidence, monitors authorized phases, and may attempt one bounded repair | Focused local-model and core actor tests; documentation review; no build or install |
| ARCH-021 | 2026-09-28 | Cloud developer / local tester roles | Made the cloud model the production-code and baseline-test owner; constrained the local actor to independent review, additional focused tests, one bounded repair, and explicit monitor/operations phase gates before `cloud_developer_handoff` | Focused local-model and core actor tests; documentation review; no build or install |
| ARCH-015 | 2026-09-28 | Truncated actor context | Added a distinct anchored `replace_excerpt` contract with excerpt-hash, live-file containment, stale-target, target-ambiguity, and rendered-line ambiguity checks while preserving complete-context replacements | 50 local-model tests; 5 focused core renderer tests; no build or install |
| DOC-001  | 2026-09-19 | Repository knowledge   | Established the architecture/feature map and living tracker/progress notes at baseline `78245b47a`                                      | Source-tree inspection and Markdown review              |
| ARCH-001 | 2026-09-19 | Hybrid local execution | Extended the architecture with Hugging Face discovery, secure downloads, a model registry, backend compatibility, and runtime switching | Design review and Markdown formatting                   |
| ARCH-002 | 2026-09-19 | Local-model foundation | Added typed storage configuration, deterministic path precedence, generated schema support, effective runtime paths, and focused tests  | 9 crate tests; config and CLI integration suites        |
| ARCH-003 | 2026-09-20 | Hugging Face lifecycle | Added remote inspection and revision resolution plus resumable, quota-limited, verified download, registry listing, and safe removal    | 15 crate tests; CLI build; live public-model inspection |
| ARCH-004 | 2026-09-21 | Cloud/local analysis router | Routed large deterministic command output to Qwen in LM Studio while retaining cloud ownership and the raw artifact | Installed release; real 220,000-byte log workload |
| ARCH-005 | 2026-09-21 | Hybrid usage statistics | Added persistent local-offload accounting and combined it with server-authoritative five-hour and weekly usage | Real ledger event: 55,000 raw versus 178 forwarded estimated tokens |
| ARCH-006 | 2026-09-21 | Live hybrid usage HUD | Added a responsive, right-aligned footer HUD for five-hour/weekly remaining and approximate locally saved tokens | Focused render test; TUI compile; installed release; real ledger refresh path |
| ARCH-007 | 2026-09-26 | Structured planner/actor protocol | Enforced the cloud planner's local-actor handoff with a strict function schema and passed only the decoded assignment into the actor system prompt | Focused tool-schema test and cloud-to-actor integration test |
| ARCH-008 | 2026-09-26 | Local actor implementation and tests | Converted planner-authorized actor patches and test commands into a reviewed execution plan for the normal permission-aware tools | 34 local-model tests; end-to-end patch and command integration test |
| ARCH-009 | 2026-09-26 | Local debugging and replanning | Persisted completed actor attempts through rollout history; after three failures cloud diagnoses and delegates a materially revised assignment with a new task ID back to the actor, while transport failures escalate immediately | Four focused core tests, including shutdown/resume, structured replan action, and transport failure coverage |
| ARCH-010 | 2026-09-26 | Session token-savings display | Baselined the persistent analysis ledger at TUI startup and displayed approximate current-session local savings in the footer and `/status` | 35 local-model tests; focused TUI snapshot reviewed and accepted |
| ARCH-011 | 2026-09-26 | Actor usage telemetry | Captured backend-reported local-actor usage in a separate ledger and reported current-session calls/input/output without adding unlike token streams to cloud savings | 38 local-model tests; 4 focused core tests; focused TUI snapshot reviewed and accepted |
| ARCH-012 | 2026-09-26 | Repeatable local CLI installation | Added a Windows local-development installer that builds and hash-verifies `codex-local` with its Code Mode host, uses verified Codex V8 artifacts, and updates user PATH idempotently | Installed twice safely; fresh Git Bash resolved both executables and host help passed |
| ARCH-013 | 2026-09-26 | Structured actor edits | Replaced actor-authored patch grammar with bounded hashed context, structured add/replace edits, trusted patch rendering, and retryable safe validation failures | 39 local-model tests; 7 focused core tests |
| ARCH-014 | 2026-09-28 | Live actor validation | Exercised an existing-file unit-test task through live LM Studio inference; schema restriction forced `replace`, cloud review rejected attempt 1, and actor retry 2 produced an accepted edit and compliant test command | Installed release hash match; reviewed actor edit applied; 48 local-model tests passed |
| ARCH-016 | 2026-09-28 | Actor-model context and loading | Added context-length input and validation, LM Studio maximum enforcement, input lock plus loading/failure states, and deterministic single-instance loading with GPU maximum and parallelism one | 12 focused TUI tests; five reviewed snapshots; 48 local-model tests; installed release hash match; live 32,768-context actor call |
| ARCH-017 | 2026-09-28 | Usage HUD reset and savings | Added the next available five-hour/weekly reset time beside remaining limits and current-session approximate locally saved tokens, including a visible zero-savings state when rate limits are available | 3 focused TUI tests; 2 reviewed inline snapshots; 7 focused core tests; broader TUI suite reached 4,917 passes |
| ARCH-018 | 2026-09-28 | Background command panel | Added a bounded top-right quarter panel for active unified-exec background commands and recent output, hidden on small terminals, and refined actor handoff guidance from observed failures | 3 focused TUI tests; 2 reviewed snapshots; no build or install |
| ARCH-019 | 2026-09-28 | Local actor usage HUD | Added current-session local actor call, input-token, and generated-token totals to the footer HUD while preserving analysis savings as a separate estimate | 4 focused TUI tests; 3 reviewed inline snapshots; no build or install |
| ARCH-020 | 2026-09-28 | Context and cloud-savings HUD | Added authoritative active cloud-conversation context size and model limit, clarified avoided cloud input as `Cloud saved`, refreshed the HUD on token updates, and promoted complete-context atomic actor handoffs for small mechanical edits | 5 focused TUI tests; 4 reviewed inline snapshots; 5 focused core actor-edit tests; no build or install |

## Risks and constraints

| ID       | State    | Risk or constraint                                                                                                                                                | Mitigation / trigger                                                                            |
| -------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| RISK-001 | Watching | Rust, MSVC build tools, `just`, `cargo-nextest`, Bazelisk, and PowerShell 7 run in the VS developer shell; `uv` and `dotslash` remain unavailable on PATH | Rust formatting succeeds; install or expose the missing tools before relying on repo-wide Python and Bazel/Starlark formatting |
| RISK-002 | Watching | `main` is high-churn and feature stages can drift quickly                                                                                                         | Refresh the baseline commit and feature counts when beginning a new work item                   |
| RISK-003 | Watching | Several central orchestration modules are already large and high-touch                                                                                            | Prefer new focused modules and keep non-mechanical changes below the repository's size guidance |
| RISK-004 | Watching | Changes may cross CLI, app-server v2, internal protocol, persistence, SDK, or config compatibility boundaries                                                     | Perform explicit breaking-change review whenever one of those surfaces changes                  |
| RISK-005 | Watching | Supported deployments can split app-server and exec-server across different operating systems                                                                     | Use remote-executor-aware builders and cover foreign path/OS behavior in integration tests      |
| RISK-006 | Resolved | The first `codex-core` test build exhausted the former 16 GB Windows memory/page-file capacity while compiling in parallel (OS error 1455); the machine now has 48 GB RAM | Use Cargo's default parallelism; reintroduce a scoped job cap only if measured memory pressure returns |
| RISK-007 | Open     | One existing Windows doctor snapshot fails to normalize its temporary config path after the repository move; it is unrelated to local-model behavior              | Diagnose separately; do not accept a machine-specific temporary path into the snapshot          |
| RISK-008 | Resolved | LM Studio initially timed out while waking its desktop daemon                                                                                                      | Server is now reachable; existing Qwen models are visible and strict structured chat completion succeeded                |
| RISK-009 | Open     | The normal Windows workspace sandbox fails to launch PowerShell with `CreateProcessWithLogonW failed: 2`; the configured legacy `[sandbox]` table is also ignored | Diagnose Windows sandbox prerequisites and migrate the user setting before relying on sandboxed non-interactive workloads |
| RISK-010 | Resolved | Preloading Qwen and then loading it again through Codex Local created duplicate LM Studio instances and left the second instance with only about 3 GiB of VRAM | Use context `32768`; unload every matching model-key instance before loading exactly one with `--gpu max --parallel 1`. LM Studio reports 33,024 after its 256-token runtime overhead |

## Decisions

| ID      | Date       | Decision                                                                   | Rationale                                                                                                                       | Revisit when                                                            |
| ------- | ---------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| DEC-001 | 2026-09-19 | Keep project-memory files in `docs/internal/`                              | Satisfies the requested `docs` location while distinguishing internal engineering state from public product documentation       | The repository adopts a standard planning or engineering-notes location |
| DEC-002 | 2026-09-19 | Use source links and capability families instead of copying every flag/API | Detailed copies drift rapidly; registries and generated schemas remain authoritative                                            | A generated inventory replaces this manual map                          |
| DEC-003 | 2026-09-19 | Resolve versioned local-model artifacts through capability profiles        | Decouples tasks from one model/backend and makes Hugging Face downloads and run reproduction safe                               | Model lifecycle implementation reveals a narrower viable contract       |
| DEC-004 | 2026-09-19 | Keep the selected cloud model as the authoritative problem-solving brain   | Local models reduce cloud context cost by scanning high-volume evidence, but do not own diagnosis, edits, safety, or acceptance | Measured quality shows a specific bounded workload can safely expand    |

## New-item template

Copy one row into **Active work** and fill every field:

```text
| AREA-NNN | Ready | <crate/surface> | <observable result> | <owner> | <single next action> |
```

For each item:

1. Link the request, issue, or source evidence.
2. List affected crates and integration surfaces before implementation.
3. Record required targeted tests and schema/snapshot regeneration.
4. Move it to **Completed** only after validation is recorded in
   [progress.md](progress.md).
5. Update [features.md](features.md) only if the repository map or capability
   inventory materially changed.
