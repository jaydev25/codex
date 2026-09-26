# Repository features and architecture

> Internal engineering notes. This is a navigational map, not public product
> documentation or a promise that every compiled capability is generally
> available.

## Baseline

- Snapshot date: 2026-09-19
- Git baseline: `78245b47a` on `dev` (`main` points to the same commit)
- Product: OpenAI Codex CLI and its local agent runtime, services, SDKs, and
  release tooling
- Primary implementation: Rust 2024 workspace under `codex-rs/`
- Other maintained surfaces: TypeScript and Python SDKs, the npm launcher,
  scripts, Bazel metadata, and GitHub Actions workflows
- Scale at this snapshot: 154 Cargo package manifests, approximately 4,481
  Rust files, 750 TypeScript files, 169 Python files, and 1,249 Rust snapshots

Treat the source tree and generated schemas as authoritative. This file is a
map for finding that authority quickly.

## Product surfaces

| Surface             | Purpose                                                                                 | Primary location                                          |
| ------------------- | --------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `codex`             | Multitool command that defaults to the interactive terminal UI                          | `codex-rs/cli/src/main.rs`                                |
| Interactive TUI     | Conversation, transcript, approvals, diffs, session browsing, settings, and realtime UI | `codex-rs/tui/`                                           |
| `codex exec`        | Scriptable/non-interactive agent runs and reviews                                       | `codex-rs/exec/`                                          |
| App server          | JSON-RPC service used by first-party clients and the current TUI architecture           | `codex-rs/app-server/`                                    |
| App-server protocol | Versioned external API, generated TypeScript, and JSON schema fixtures                  | `codex-rs/app-server-protocol/`                           |
| Exec server         | Standalone execution/environment service                                                | `codex-rs/exec-server/`, `codex-rs/exec-server-protocol/` |
| TypeScript SDK      | Spawns the CLI and exchanges structured JSONL events                                    | `sdk/typescript/`                                         |
| Python SDK          | Starts threads, runs turns, streams events, and controls workspace access               | `sdk/python/`, `sdk/python-runtime/`                      |
| npm launcher        | Publishes the `@openai/codex` executable wrapper                                        | `codex-cli/`                                              |

The CLI also exposes login/logout, MCP and plugin management, app-server and
remote-control commands, sandbox diagnostics, session resume/fork/archive/
delete operations, cloud tasks, completion generation, feature inspection,
and internal release/debug utilities.

The hybrid local-model branch additionally exposes a `local_actor` tool when
local analysis is enabled. It forwards a structured task to the configured
loopback model and returns patch/test proposals for cloud review and normal
permission-aware tool execution. Core derives a reviewed execution plan only
for planner-approved `apply_patch` and `exec_command` operations; the cloud
invokes those normal tools and verifies their evidence. Same-task retries are
bounded to three local attempts, then the original task is returned to cloud.
Completed call/output pairs rebuild the bounded attempt state from rollout
history after process resume. Cloud diagnoses a three-attempt failure and
hands a materially revised assignment with a new task ID back to the actor;
it does not default to writing the implementation itself. The actor never
receives direct execution authority. See [architecture.md](architecture.md) and
[trackers.md](trackers.md) for the design and validation record.

The interactive footer and `/status` report approximate cloud-input tokens
saved during the current TUI process. The value is derived by subtracting the
persistent ledger at startup from its latest value, so lifetime totals from
earlier sessions are not presented as current-session savings. This counter
currently covers successful large-output local-analysis offloads; local-actor
inference usage is tracked as a separate follow-up until it can be reported
without treating unlike token streams as equivalent.

## Runtime architecture

The main request path is:

1. `codex-rs/cli` parses the command and selects TUI, exec, app-server, or a
   utility command.
2. The TUI or app-server constructs configuration, authentication, model,
   plugin, and execution dependencies.
3. `codex-rs/core` owns the agent/thread orchestration, Responses API loop,
   context construction, tool dispatch, approvals, compaction, and turn state.
4. `codex-rs/protocol` carries internal cross-crate types and events;
   `codex-rs/app-server-protocol` defines the external JSON-RPC boundary.
5. Tool and extension crates perform filesystem, process, MCP, web, memory,
   image, collaboration, and other operations through sandbox and policy
   layers.
6. Rollout, history, thread-store, and state crates persist sessions and
   metadata; UI and API adapters project those events back to clients.

Important boundary rule: app-server API development belongs in v2. Changes to
the app-server API, raw response item events, CLI arguments, configuration
loading, or rollout resumption are integration-surface changes and require
compatibility review.

## Capability map

| Capability                    | What exists                                                                                                                                                                            | Useful source anchors                                                                                                                                                 |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Agent loop                    | Threads, turns, streaming Responses API handling, tool calls, approvals, interruption, steering, and compaction                                                                        | `codex-rs/core/src/`, `codex-rs/protocol/src/turn_input.rs`                                                                                                           |
| Session lifecycle             | Start, resume, fork, queue, archive, unarchive, delete, migrate, persist, and recover                                                                                                  | `codex-rs/cli/src/main.rs`, `codex-rs/thread-store/`, `codex-rs/rollout/`, `codex-rs/history/`                                                                        |
| Interactive UX                | Composer, transcript, diffs, task status, model/settings pickers, approvals, mentions, voice, and session browsing                                                                     | `codex-rs/tui/src/`                                                                                                                                                   |
| Non-interactive automation    | Exec, review, JSONL events, structured output, and SDK embedding                                                                                                                       | `codex-rs/exec/`, `sdk/typescript/`, `sdk/python/`                                                                                                                    |
| Configuration                 | Layered configuration, requirements, profiles, schema generation, and live app-server config operations                                                                                | `codex-rs/config/`, `codex-rs/core/src/config/`, `codex-rs/core/config.schema.json`                                                                                   |
| Authentication and models     | ChatGPT/API-key login, keyring-backed secrets, workload identity, cloud model selection, provider abstractions, local-model storage, a versioned artifact registry, and CLI inspection | `codex-rs/login/`, `codex-rs/keyring-store/`, `codex-rs/models-manager/`, `codex-rs/model-provider/`, `codex-rs/local-models/`, `codex-rs/cli/src/local_model_cmd.rs` |
| Safe execution                | Approval policies, exec policy, command parsing, PTY execution, filesystem permissions, network policy, and platform sandboxes                                                         | `codex-rs/execpolicy/`, `codex-rs/sandboxing/`, `codex-rs/linux-sandbox/`, `codex-rs/windows-sandbox-rs/`, `codex-rs/mxc-sandbox/`                                    |
| Tools                         | Shell/unified exec, patching, image viewing, file search, sleep, permission requests, and dynamic tool registration                                                                    | `codex-rs/tools/`, `codex-rs/apply-patch/`, `codex-rs/file-search/`, `codex-rs/core/src/tools/`                                                                       |
| Extensibility                 | MCP clients/servers, connectors/apps, plugins, skills, hooks, tool discovery, and dependency prompting                                                                                 | `codex-rs/codex-mcp/`, `codex-rs/connectors/`, `codex-rs/plugin/`, `codex-rs/skills/`, `codex-rs/hooks/`, `codex-rs/ext/`                                             |
| Collaboration                 | Agent spawning/routing, shared goals, queues, messages, agent roles, graph storage, and Guardian review                                                                                | `codex-rs/core/src/agent/`, `codex-rs/agent-roles/`, `codex-rs/agent-graph-store/`, `codex-rs/ext/goal/`, `codex-rs/ext/guardian-v2/`                                 |
| Memory and context            | Project/user memories, context fragments, token/rollout budgets, history notes, and external-agent migration                                                                           | `codex-rs/memories/`, `codex-rs/context-fragments/`, `codex-rs/ext/memories/`, `codex-rs/external-agent-migration/`                                                   |
| Media and realtime            | Local images, image generation, audio/voice, and WebRTC realtime conversations                                                                                                         | `codex-rs/utils/image/`, `codex-rs/ext/image-generation/`, `codex-rs/voice-host/`, `codex-rs/realtime-webrtc/`                                                        |
| Services and remote workflows | App-server daemon, remote control, cloud tasks, code-mode host, proxying, and tunnels                                                                                                  | `codex-rs/app-server-daemon/`, `codex-rs/cloud-tasks/`, `codex-rs/code-mode-host/`, `codex-rs/network-proxy/`, `codex-rs/tcp-tunnel/`                                 |
| Diagnostics and observability | Doctor command, tracing, OpenTelemetry, analytics, feedback, rollout tracing, and state recovery                                                                                       | `codex-rs/cli/src/doctor.rs`, `codex-rs/otel/`, `codex-rs/analytics/`, `codex-rs/rollout-trace/`, `codex-rs/diagnostics/`                                             |

## Feature-flag lifecycle snapshot

`codex-rs/features/src/lib.rs` is the source of truth for feature keys, stages,
and defaults. At the baseline commit it contains:

| Stage              | Count | Meaning                                                                                   |
| ------------------ | ----: | ----------------------------------------------------------------------------------------- |
| Stable             |    45 | Supported flag or gate; many are enabled by default                                       |
| Experimental       |     3 | User-visible experiments: plan history, daemon auto-start, and network proxy              |
| Under development  |    57 | Incomplete/internal work that may behave unpredictably                                    |
| Deprecated         |     3 | Compatibility paths still recognized but discouraged                                      |
| Removed            |    39 | No-op or compatibility keys retained for old configuration                                |
| Platform-dependent |     1 | Prevent-idle-sleep is experimental on macOS/Linux/Windows and under development elsewhere |

Notable stable families include unified execution, memory, hooks, worktrees,
multi-agent routing, apps/plugins, image generation, goals, realtime voice,
Guardian approvals, and workspace dependencies. The registry deliberately
retains removed keys so old configuration continues to parse; a key's presence
does not mean its old behavior still exists.

Use `codex features` and the registry when changing flags. Update this summary
whenever a `FeatureSpec` is added or changes stage.

## Repository map

| Path                                    | Role                                                                                                   |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `codex-rs/`                             | Rust workspace and primary product implementation                                                      |
| `sdk/typescript/`                       | Published TypeScript SDK                                                                               |
| `sdk/python/`                           | Published Python SDK                                                                                   |
| `sdk/python-runtime/`                   | Python runtime/package support                                                                         |
| `codex-cli/`                            | npm executable package                                                                                 |
| `docs/`                                 | Repository documentation stubs and these internal engineering notes; official user docs live elsewhere |
| `.github/workflows/`                    | CI, release, policy, SDK, Bazel, and V8 workflows                                                      |
| `scripts/`                              | Packaging, release, schema, and maintenance automation                                                 |
| `bazel/`, `BUILD.bazel`, `MODULE.bazel` | Bazel build integration and lock state                                                                 |
| `patches/`, `third_party/`              | Vendored integration patches and notices                                                               |
| `.codex/skills/`                        | Repository-specific Codex workflow skills                                                              |

## Development and validation

- Root `justfile` recipes run in `codex-rs/` by default.
- Format Rust changes with `just fmt`.
- Run targeted tests with `just test -p <crate>`; do not invoke `cargo test`
  directly.
- Run `just fix -p <crate>` before finalizing a large Rust change, after tests.
- Changes in common/core/protocol require the full `just test` suite after
  targeted tests, but ask before starting that full suite.
- App-server wire changes require `just write-app-server-schema` and protocol
  tests; experimental wire changes also require the experimental schema.
- Config type changes require `just write-config-schema`.
- Rust dependency changes require `just bazel-lock-update`.
- User-visible TUI changes require reviewed and accepted `insta` snapshots.
- Cross-platform behavior must account for Linux, macOS, Windows, and cases
  where app-server and exec-server run on different operating systems.

## Maintenance rules for this file

Update this file when a change adds/removes a product surface, moves an
architectural boundary, adds a major crate, or changes feature-stage counts.
Record the work item in [trackers.md](trackers.md) and append the outcome and
validation evidence to [progress.md](progress.md). Do not turn this file into a
release changelog or duplicate generated/API documentation.
