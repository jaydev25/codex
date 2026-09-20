# System Architecture: Local GPU Agent Loop with Codex Orchestrator

This document defines an asynchronous, multi-model agent system that uses
the user-selected cloud Codex model as the authoritative reasoning and
implementation agent, while models running on a local RTX 3090 24 GB GPU
perform high-volume support work such as test-output analysis, log scanning,
failure clustering, and issue extraction.

The local execution model is not fixed. Users can discover, download, register,
activate, and switch compatible models from Hugging Face or other supported
local sources without changing the orchestration workflow.

---

## 1. Architectural Strategy and Token Optimization

### 1.1 The Macro/Micro Split

To minimize external API token consumption and control costs, the system
separates planning from execution:

- **Cloud Brain (user-selected Codex model, for example 5.6 Sol):** Owns
  planning, diagnosis, implementation decisions, patch generation, safety
  decisions, and final verification. It remains authoritative throughout a
  run rather than handing problem ownership to a local model.
- **Local Support Worker (RTX 3090 24 GB):** Handles token-heavy, bounded,
  evidence-processing workloads: scanning test output and logs, grouping
  repeated failures, extracting likely causes and source references, and
  producing compact structured summaries. It cannot approve changes, mutate
  the plan, or make the final diagnosis.
- **Model Manager:** Resolves a requested capability profile to an installed
  local model, downloads missing Hugging Face artifacts when authorized, and
  starts the compatible inference backend.

### 1.2 Multi-Model Token-Saving Loop

1. **Plan and implement in cloud:** The selected cloud model creates the plan
   and proposed code changes using the repository context and user intent.
2. **Run deterministic tools:** Codex invokes approved tests, linters, builds,
   and scanners in the normal sandbox or execution environment.
3. **Resolve a local analyst:** For sufficiently large outputs, the Model
   Manager selects an installed local model or asks the user to select/download
   one. Small outputs bypass local inference entirely.
4. **Analyze locally:** The local model receives bounded artifacts and returns
   a schema-validated evidence digest containing failure groups, source spans,
   test names, commands, exit status, hypotheses, and confidence.
5. **Solve in cloud:** The cloud model evaluates the digest, requests raw
   excerpts when evidence is incomplete or confidence is low, and owns the
   diagnosis and next patch.
6. **Repeat selectively:** Raw high-volume artifacts stay local; only bounded
   evidence and requested excerpts consume cloud context.

---

## 2. Local Hardware and Model Configuration

### 2.1 Hardware Target

The initial target is a single NVIDIA RTX 3090 with 24 GB VRAM. Model fit must
be calculated from the selected artifact, quantization, context length, KV
cache type, concurrency, and backend overhead. Published parameter counts or
nominal quantization alone are not sufficient guarantees that a model will fit.

The Model Manager must perform a preflight estimate and preserve a configurable
VRAM safety margin before loading a model. It should reject or suggest a smaller
context/model when the estimate exceeds the available budget.

### 2.2 Supported Inference Backends

The architecture uses a backend adapter interface and may support:

- **vLLM:** High-throughput serving for compatible Transformers,
  SafeTensors/AWQ/GPTQ, or other backend-supported artifacts.
- **llama.cpp:** Efficient GGUF execution with configurable GPU-layer offload.
- **Ollama:** Simple local lifecycle and an OpenAI-compatible endpoint,
  including imported or derived model definitions when supported.
- **LM Studio:** User-managed local models exposed through its local API.

Every backend adapter must implement health checking, model load/unload,
capability reporting, context-limit reporting, and an OpenAI-compatible
inference endpoint or an internal equivalent.

### 2.3 Hugging Face Model Discovery and Download

Hugging Face is a first-class model source. The system must provide operations
to:

- Search or resolve a repository ID and inspect available revisions and files.
- Filter artifacts by supported architecture, format, quantization, size,
  context length, and selected backend.
- Display download size, estimated disk usage, expected VRAM range, license,
  gating status, and whether custom model code is requested.
- Authenticate for private or gated repositories using a user-provided token
  stored through the configured secret backend, never in project files or logs.
- Require explicit acceptance when a repository has gated access or license
  terms that need acknowledgement.
- Pin downloads to an immutable commit revision rather than silently tracking
  a moving branch.
- Download resumably into a content-addressed cache, using temporary partial
  files and atomic promotion after integrity checks pass.
- Verify available hashes/metadata, reject unexpected file types, and record
  the exact artifact set used by a run.
- Support cancellation, retry, progress events, storage quotas, and safe model
  removal when no active runtime is using the artifacts.

The downloader must not execute repository-provided Python or enable
`trust_remote_code` by default. Models requiring custom code must be clearly
identified and require an explicit policy decision before use.

### 2.4 Local Model Registry

Downloaded and externally managed models are normalized into a local registry.
Each entry contains at least:

```json
{
  "model_id": "qwen-coder-local",
  "source": {
    "kind": "hugging_face",
    "repository": "organization/model-name",
    "revision": "immutable-commit-sha"
  },
  "artifacts": {
    "format": "gguf",
    "quantization": "Q4_K_M",
    "local_path": "model-cache/content-id"
  },
  "runtime": {
    "backend": "llama_cpp",
    "context_length": 8192,
    "gpu_layers": "auto"
  },
  "capabilities": ["code_generation", "tool_use"],
  "license": {
    "identifier": "model-license-id",
    "accepted_at": "timestamp-or-null"
  }
}
```

Registry entries are versioned records. Updating to another Hugging Face
revision creates a new resolved artifact record so existing runs remain
reproducible.

### 2.5 Model Storage Configuration

The download root is configured in the user's Codex `config.toml`:

```toml
[local_models.storage]
models_dir = "E:\\AI\\Models\\codex"
temp_dir = "E:\\AI\\Models\\codex\\.downloads"
max_disk_gb = 500

[local_models.analysis]
enabled = true
model_id = "local-log-analyst"
min_output_bytes = 65536
max_input_bytes = 2097152
backend_url = "http://127.0.0.1:1234/v1"
backend_model = "local-log-analyst"
```

`models_dir` is the durable artifact root, `temp_dir` holds partial downloads,
and the registry is stored at `<models_dir>/registry.json`. If `temp_dir` is
omitted it defaults to `<models_dir>/.downloads`.

The model-root precedence is:

1. A command-specific override, once the model-management CLI is implemented.
2. The `CODEX_LOCAL_MODELS_DIR` environment variable.
3. `[local_models.storage].models_dir` in `config.toml`.
4. `<CODEX_HOME>/models`.

Storage configuration, runtime path resolution, the versioned registry,
inspection, download, listing, and safe removal are implemented. Analysis is
disabled by default. When enabled, only registered `model_id` values are
eligible; outputs below `min_output_bytes` bypass local analysis, and inputs are
bounded by `max_input_bytes`. Runtime activation remains a later lifecycle
stage. The initial analysis adapter accepts only HTTP(S) loopback URLs and an
OpenAI-compatible chat-completions surface, allowing an explicitly served model
to analyze evidence without permitting raw logs to be sent to a remote host.
Inference requests use the OpenAI-compatible `json_schema` response format with
a strict evidence schema; Codex still validates the decoded response and every
citation before placing the digest in cloud-model context.
Completed `exec_command` calls are classified when analysis is enabled. Eligible
large output is captured under `<CODEX_HOME>/analysis-artifacts`, analyzed with
a 30-second endpoint timeout, and replaced in the cloud-model context only after
the digest passes schema and citation validation. Every bypass or analysis
failure preserves the original model-facing output.
Commands that remain active after their initial yield are evaluated when a
later `write_stdin` call observes final completion. Routing telemetry contains
only outcome/reason labels and artifact byte counts; it excludes command text,
raw output, artifact paths, findings, and model responses.

### 2.6 Model Selection and Switching

Tasks request capabilities, not hard-coded model names. A selection policy
matches the task against registered models using:

- Required capabilities such as code generation, structured output, tool use,
  vision, or long context.
- Backend and artifact compatibility.
- Estimated VRAM and RAM use at the requested context length.
- User preference, model allowlists/denylists, and offline-only policy.
- Recent measured reliability, latency, and validation success rate.

Users may override the selected model globally, per project, per run, or per
task. Switching models must drain active requests, unload the previous runtime
when necessary, start the new backend/model, perform a health probe, and only
then route work to it. Failure to load must leave the prior healthy model
available when possible.

Proposed user-facing operations are:

```text
codex local-model search <query>
codex local-model inspect <hugging-face-repo-or-model-id>
codex local-model download <repo> [--revision <sha>] [--format <format>]
codex local-model list
codex local-model status [--json]
codex local-model activate-lm-studio <model-id> [--dry-run]
codex local-model activate <model-id>
codex local-model serve <model-id>
codex local-model remove <model-id>
```

`codex local-model path`, `codex local-model list [--json]`, `codex local-model
status [--json]`, read-only remote
inspection, safe removal, and the initial single-artifact download path are
implemented. `status` probes `<backend_url>/models` with a five-second timeout,
requires a loopback endpoint, lists the models exposed by the server, and exits
with an error when the configured `backend_model` is unavailable.
`activate-lm-studio` supports registered GGUF artifacts: it copies the artifact
into LM Studio without removing the Codex-managed source, loads it with maximum
GPU offload under `backend_model`, starts a loopback-only server on the port in
`backend_url`, and verifies the model through `/models`. `--dry-run` prints the
three safely quoted `lms` commands without changing runtime state. The concrete
download contract is:

```text
codex local-model download <owner/repository> <filename> \
  --model-id <local-id> \
  --revision <branch-tag-or-commit> \
  [--format gguf] [--quantization Q4_K_M] [--sha256 <digest>]
```

Authentication is read from `HF_TOKEN`; tokens are not accepted as command-line
arguments. `local-model inspect <owner/repository> [--revision <revision>]`
returns the resolved commit and repository metadata without downloading model
weights. Downloads resolve the supplied branch, tag, or commit first, then use
the resulting immutable 40-character SHA for the artifact request and registry.
They resume from staging files when the server honors byte ranges, verify the
optional SHA-256 digest, promote the completed artifact, and update the
registry. `[local_models.storage].max_disk_gb` is enforced against existing
artifacts, staging files, and incoming bytes. `local-model remove` deletes only
registered artifact paths beneath the configured model directory; an outside
path is rejected without changing the registry. Whole-repository snapshots,
search, and backend-neutral activate/serve remain later lifecycle stages.

---

## 3. Core Component Blocks

The system contains six logical components. They may initially run in one
process; they are boundaries of responsibility, not a requirement for six
independent network services.

```text
    +----------------------------------------+
    |           Codex Orchestrator           |
    |          (High-Level Planner)          |
    +-------------------+--------------------+
                        | Schema-validated blueprint
                        v
    +-------------------+--------------------+
    |        Hybrid Loop Coordinator         |
    |       (State, Policy, Scheduling)      |
    +----------+------------------+----------+
               |                  |
               v                  v
    +----------+---------+  +-----+------------------+
    | Model Manager      |  | Local Model Registry  |
    | HF Download/Serve  |  | Metadata/Artifacts    |
    +----------+---------+  +------------------------+
               | OpenAI-compatible inference
               v
    +----------+-----------------------------+
    |          Local Micro-Agent             |
    |       (Patch Generation/Critic)        |
    +-------------------+--------------------+
                        | Candidate patch
                        v
    +-------------------+--------------------+
    |   Isolated Worktree and Sandbox        |
    |      (Apply, Execute, Validate)         |
    +-------------------+--------------------+
                        | Bounded result and diagnostics
                        +-----------> Coordinator
```

### 3.1 Orchestrator Bridge

- Requests a versioned JSON blueprint using a strict output schema.
- Supplies repository constraints and relevant context without requesting full
  source-file reproduction.
- Accepts structured escalation reports and returns plan mutations.
- Treats local summaries as untrusted evidence and retains ownership of
  planning, diagnosis, edits, and final acceptance.

### 3.2 Hybrid Loop Coordinator

- Owns the durable state machine for plans, tasks, attempts, and escalation.
- Enforces dependency ordering, concurrency limits, cancellation, timeouts,
  token budgets, and fallback to raw evidence when local analysis is weak.
- Resolves an analysis profile through the Model Manager only when artifact
  size and estimated cloud-token savings justify local inference.
- Emits auditable progress events and supports safe resume after interruption.

### 3.3 Model Manager and Hugging Face Adapter

- Implements discovery, authentication, download, verification, registration,
  load/unload, and health monitoring.
- Maps artifact formats to compatible inference backends.
- Keeps secrets out of prompts, logs, blueprints, and registry metadata.
- Exposes progress and actionable errors for disk, network, license, format,
  backend, and VRAM failures.

### 3.4 Local Analysis Worker

- Routes bounded test, build, lint, scanner, and log artifacts to the selected
  local model endpoint.
- Builds bounded prompts from the analysis question and immutable artifact
  references without granting repository-write authority.
- Requires schema-validated evidence output with citations to artifact spans,
  confidence, and explicit unknowns.
- Records the exact model revision, backend, parameters, and prompt-template
  version used for reproducibility.

### 3.5 Isolated Worktree and Sandbox

- Applies candidate patches to a disposable Git worktree or equivalent staged
  workspace, never directly to the user's working tree.
- Runs only allowlisted commands with explicit filesystem, network, CPU,
  memory, output-size, and wall-clock limits.
- Captures stdout, stderr, exit status, changed files, and test results.
- Promotes validated changes to the target workspace through a reviewed,
  recoverable operation.

### 3.6 Cloud-Owned Evaluation Loop

- Classifies failures as generation/schema, patch conflict, compile, test,
  lint, timeout, sandbox/policy, infrastructure, or architectural failures.
- Sends large diagnostic artifacts to a local analyst and keeps raw artifacts
  addressable by stable IDs.
- Returns compact evidence to the cloud brain; the cloud brain may request raw
  excerpts and determines every repair.
- Bypasses or rejects local analysis when output is small, evidence references
  are invalid, confidence is below policy, or the local backend is unhealthy.

---

## 4. Execution State Machine

```text
[Start]
   |
   v
[Cloud brain plans and creates a candidate change]
   |
   v
[Run deterministic validation commands]
   | output large enough for local analysis?
   +---- no ----> [Cloud brain reads result directly]
   |
   v yes
[Resolve local analysis profile]
   | installed and compatible?
   +---- no ----> [User authorizes/selects HF download]
   |                   |
   |                   v
   |              [Download, verify, register]
   |                   |
   +<------------------+
   |
   v
[Load model and health-check backend]
   |
   v
[Local model scans logs/test output]
   |
   v
[Return cited evidence digest to cloud brain]
   |
   v
[Cloud brain diagnoses and decides next change]
   | evidence sufficient?
   +---- no ----> [Request raw excerpts or bypass local analyst] --+
   |                                                            |
   +<------------------------------------------------------------+
   |
   v
[Cloud brain patches; sandbox validates]
   | pass
   v
[Cloud brain accepts final result]
   v
[Next task or complete]
```

Every transition is persisted with a run ID, task ID, attempt number, model
artifact identity, and repository revision. Resume must not repeat a promotion
or assume a partially downloaded model is complete.

---

## 5. Prompting and Data Structures

### 5.1 Macro Blueprint

Codex returns JSON conforming to a versioned schema similar to:

```json
{
  "schema_version": 1,
  "project_goal": "String",
  "repository_revision": "git-commit-sha",
  "steps": [
    {
      "task_id": "task-1",
      "depends_on": [],
      "target_paths": ["path/to/target.rs"],
      "objective": "Detailed implementation criteria",
      "interfaces": ["Explicit signatures and types"],
      "constraints": ["Repository and security constraints"],
      "acceptance_commands": ["approved targeted test command"],
      "executor_profile": {
        "required_capabilities": ["code_generation", "structured_output"],
        "minimum_context_length": 8192,
        "preferred_model_id": null
      }
    }
  ]
}
```

The schema is descriptive and stable. Token reduction should come from
excluding irrelevant context and compact JSON serialization, not obscure field
names.

### 5.2 Local Micro-Task Input

```text
[LOCAL EXECUTOR TASK]
Run: {{run_id}}
Task: {{task_id}}
Repository revision: {{repository_revision}}
Allowed paths: {{target_paths}}
Objective: {{objective}}
Interfaces: {{interfaces}}
Constraints: {{constraints}}
Acceptance commands: {{acceptance_commands}}

Return only a response matching the candidate-patch schema. Do not access or
modify paths outside the allowed set.
```

The response schema should contain patch operations, assumptions, and suggested
validation commands. Commands are suggestions until independently approved by
the coordinator's policy layer.

### 5.3 Escalation Report

```json
{
  "run_id": "run-id",
  "task_id": "task-1",
  "status": "local_repairs_exhausted",
  "attempts": 3,
  "failure_class": "test",
  "diagnostic_summary": "Bounded failure explanation",
  "changed_paths": ["path/to/target.rs"],
  "validation_command": "targeted test command",
  "last_exit_code": 1,
  "requested_action": "revise_plan"
}
```

Raw logs remain local and bounded. Codex receives only the evidence needed to
make a structural decision, with secrets and unrelated source removed.

---

## 6. Optimization, Pruning, and Budget Controls

- **Context pruning:** Keep complete artifacts and bounded attempt records in
  local storage. Send the model only relevant source slices, current diffs, and
  the latest diagnostic window.
- **Prompt stability:** Version prompt templates and schemas so cached prefixes
  remain stable and changes are auditable.
- **Local repair cache:** Cache a repair only with repository revision,
  toolchain/environment fingerprint, failing command, normalized diagnostic,
  model artifact, and prompt version. Always rerun validation after applying a
  cached repair.
- **Budgets:** Enforce separate limits for commercial tokens, local tokens,
  elapsed time, repair attempts, download bytes, disk use, and GPU memory.
- **Output bounds:** Cap logs, patches, tool output, and escalation payloads.
  Store larger artifacts locally and refer to them by ID.
- **Offline mode:** Once artifacts are installed, allow policy to prohibit all
  model-network access while preserving local inference and validation.

---

## 7. Security, Trust, and Reproducibility

- Treat model cards, repository metadata, tokenizer templates, and downloaded
  artifacts as untrusted input.
- Never execute downloaded repository code during discovery or download.
- Default to formats that do not require arbitrary code execution.
- Store Hugging Face credentials in the configured secret store and redact
  authorization headers and repository URLs containing credentials.
- Pin model revisions and record artifact hashes, backend version, CUDA/runtime
  information, generation parameters, and prompt-template version per run.
- Separate model download network permission from sandboxed code-execution
  network permission.
- Require explicit confirmation before large downloads, gated-license
  acceptance, custom-code execution, or deletion of shared cached artifacts.
- Preserve an auditable record of local failures even when only summaries are
  sent to Codex.

---

## 8. Delivery Phases

### Phase 1: Storage and Deterministic Analysis Loop

- Typed model-storage configuration and deterministic path precedence.
- One cloud brain and one preconfigured local OpenAI-compatible analysis
  endpoint.
- Structured test/log evidence schema, artifact references, confidence-based
  fallback, and cloud-owned diagnosis and repair.

### Phase 2: Hugging Face Model Lifecycle

- Discovery/inspection, authenticated and resumable downloads, immutable
  revision pinning, integrity verification, local registry, and removal.
- One initial artifact/backend path, selected according to the implementation
  environment, before adding the full compatibility matrix.
- CLI/app-server progress events and model-management operations.

### Phase 3: Dynamic Selection and Multiple Backends

- Capability-based analysis-model selection, VRAM preflight, load/unload
  switching, health fallback, and adapters for additional runtimes.
- Per-project and per-task model selection policies.

### Phase 4: Optimization

- Analysis caching, parallel independent scans, measured model routing, richer
  observability, and offline analysis.

The initial implementation should live in a focused new crate or extension and
reuse existing Codex provider, app-server, worktree, sandbox, secret-storage,
and event abstractions. It should avoid adding orchestration-specific code to
`codex-core` unless an existing reusable boundary must be extended.
