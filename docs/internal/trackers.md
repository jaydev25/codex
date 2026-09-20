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
| ARCH-004 | Validate | Cloud/local analysis router | Route large test, lint, scanner, and log outputs to local analysis while the selected cloud model owns planning, diagnosis, edits, and acceptance | Codex | Download a GGUF, dry-run then execute LM Studio activation, and run one routed command |

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
| RISK-008 | Open     | LM Studio CLI is installed and the RTX 3090 is available, but no local model is installed and the CLI currently times out while waking the desktop daemon       | Open/update LM Studio, download or import a compatible model, start its API server, then run the ARCH-004 live validation |

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
