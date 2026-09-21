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
| ARCH-007 | In progress | Structured planner/actor protocol | Constrain the selected cloud model to validated JSON orchestration steps and tool-call maps; pass each assignment into the local actor system prompt | Codex | Add machine-enforced planner output; scoped planner guidance and the read-only `local_actor` tool are now wired |
| ARCH-008 | In progress | Local actor implementation and tests | Delegate implementation patches plus unit and E2E test authoring to the selected LM Studio model under existing tool permissions | Codex | Verify actor-authored apply_patch and test proposals through permission-aware cloud tool calls and acceptance evidence |
| ARCH-009 | In progress | Local debugging and escalation | Let the local actor diagnose and repair a task for at most three unsuccessful iterations, then return the original problem to the selected cloud planner | Codex | Validate three-failure integration test; persist attempt state across process resume and handle actor transport failures |

## Backlog

The backlog is intentionally empty. Do not infer a roadmap from feature flags,
TODO comments, or crate names. Add only work explicitly requested or agreed.

| ID  | Priority | Area | Desired outcome | Evidence / issue | Dependencies |
| --- | -------- | ---- | --------------- | ---------------- | ------------ |

## Completed

| ID       | Completed  | Area                   | Outcome                                                                                                                                 | Validation                                              |
| -------- | ---------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| DOC-001  | 2026-09-19 | Repository knowledge   | Established the architecture/feature map and living tracker/progress notes at baseline `78245b47a`                                      | Source-tree inspection and Markdown review              |
| ARCH-001 | 2026-09-19 | Hybrid local execution | Extended the architecture with Hugging Face discovery, secure downloads, a model registry, backend compatibility, and runtime switching | Design review and Markdown formatting                   |
| ARCH-002 | 2026-09-19 | Local-model foundation | Added typed storage configuration, deterministic path precedence, generated schema support, effective runtime paths, and focused tests  | 9 crate tests; config and CLI integration suites        |
| ARCH-003 | 2026-09-20 | Hugging Face lifecycle | Added remote inspection and revision resolution plus resumable, quota-limited, verified download, registry listing, and safe removal    | 15 crate tests; CLI build; live public-model inspection |
| ARCH-004 | 2026-09-21 | Cloud/local analysis router | Routed large deterministic command output to Qwen in LM Studio while retaining cloud ownership and the raw artifact | Installed release; real 220,000-byte log workload |
| ARCH-005 | 2026-09-21 | Hybrid usage statistics | Added persistent local-offload accounting and combined it with server-authoritative five-hour and weekly usage | Real ledger event: 55,000 raw versus 178 forwarded estimated tokens |
| ARCH-006 | 2026-09-21 | Live hybrid usage HUD | Added a responsive, right-aligned footer HUD for five-hour/weekly remaining and approximate locally saved tokens | Focused render test; TUI compile; installed release; real ledger refresh path |

## Risks and constraints

| ID       | State    | Risk or constraint                                                                                                                                                | Mitigation / trigger                                                                            |
| -------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| RISK-001 | Resolved | Rust, MSVC build tools, `just`, `cargo-nextest`, Bazelisk, PowerShell 7, `uv`, and `dotslash` are installed and repository commands run in the VS developer shell | Keep the developer-shell command documented for subsequent Windows validation                   |
| RISK-002 | Watching | `main` is high-churn and feature stages can drift quickly                                                                                                         | Refresh the baseline commit and feature counts when beginning a new work item                   |
| RISK-003 | Watching | Several central orchestration modules are already large and high-touch                                                                                            | Prefer new focused modules and keep non-mechanical changes below the repository's size guidance |
| RISK-004 | Watching | Changes may cross CLI, app-server v2, internal protocol, persistence, SDK, or config compatibility boundaries                                                     | Perform explicit breaking-change review whenever one of those surfaces changes                  |
| RISK-005 | Watching | Supported deployments can split app-server and exec-server across different operating systems                                                                     | Use remote-executor-aware builders and cover foreign path/OS behavior in integration tests      |
| RISK-006 | Resolved | The first `codex-core` test build exhausted Windows memory/page-file capacity while compiling in parallel (OS error 1455); no test assertion ran or failed        | A two-job Cargo cap compiled core and the complete CLI test graph without resource failure      |
| RISK-007 | Open     | One existing Windows doctor snapshot fails to normalize its temporary config path after the repository move; it is unrelated to local-model behavior              | Diagnose separately; do not accept a machine-specific temporary path into the snapshot          |
| RISK-008 | Resolved | LM Studio initially timed out while waking its desktop daemon                                                                                                      | Server is now reachable; existing Qwen models are visible and strict structured chat completion succeeded                |
| RISK-009 | Open     | The normal Windows workspace sandbox fails to launch PowerShell with `CreateProcessWithLogonW failed: 2`; the configured legacy `[sandbox]` table is also ignored | Diagnose Windows sandbox prerequisites and migrate the user setting before relying on sandboxed non-interactive workloads |
| RISK-010 | Open     | Live actor-protocol validation is blocked by insufficient system memory: LM Studio rejected Qwen 30B at 32,768 context and Qwen 14B failed to allocate its CPU_REPACK buffer | Free RAM or reduce model/context safely; do not override LM Studio loading guardrails on a nearly exhausted system |

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
