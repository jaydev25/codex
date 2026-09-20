# Configure and Run a Local Model

This Codex build can use a local model to summarize large test, lint, scanner,
build, and log outputs. The selected cloud model remains responsible for
planning, diagnosis, edits, tool decisions, and final verification.

Two model setups are supported:

1. Use a model already downloaded and loaded by LM Studio.
2. Download and register a GGUF through Codex, then activate it in LM Studio.

## Prerequisites

- LM Studio is installed and its local API server is running.
- The server is bound to loopback, normally `127.0.0.1:1234`.
- An OpenAI-compatible model is loaded in LM Studio.
- This repository's modified Codex binary has been built.

Build the binary from the repository:

```powershell
Set-Location E:\AI\Coding\codex\codex-rs
$env:CARGO_BUILD_JOBS = "2"
cargo build -p codex-cli --bin codex
```

The resulting development binary is:

```text
E:\AI\Coding\codex\codex-rs\target\debug\codex.exe
```

The commands below use `$codex` so they do not accidentally invoke a different
globally installed Codex:

```powershell
$codex = "E:\AI\Coding\codex\codex-rs\target\debug\codex.exe"
```

## Option A: Use an Existing LM Studio Model

### 1. Find the model identifier

List loaded LM Studio models:

```powershell
lms ps
```

Alternatively, query the OpenAI-compatible endpoint:

```powershell
Invoke-RestMethod http://127.0.0.1:1234/v1/models |
    ConvertTo-Json -Depth 6
```

Copy the exact `id` reported for the loaded Qwen Coder model. In the examples
below it is represented by `<LM_STUDIO_MODEL_ID>`.

### 2. Configure Codex

Edit `%USERPROFILE%\.codex\config.toml` and add:

```toml
[local_models.analysis]
enabled = true
model_id = "lm-studio-qwen-coder"
require_registered_model = false
min_output_bytes = 65536
max_input_bytes = 2097152
backend_url = "http://127.0.0.1:1234/v1"
backend_model = "<LM_STUDIO_MODEL_ID>"
```

`require_registered_model = false` is the explicit opt-in for a model managed
outside Codex. It does not permit remote inference: the endpoint must still be
HTTP(S) loopback. Keep the default value `true` for Codex-managed downloads.

`min_output_bytes` controls when offloading begins. The example sends output of
64 KiB or more to the local model. `max_input_bytes` limits each local prompt to
2 MiB while retaining the complete raw artifact for cloud verification.

### 3. Verify the connection

```powershell
& $codex local-model status
& $codex local-model status --json
```

The result should report `available: yes` for `backend_model`.

### Select the actor model in the TUI

When local analysis is enabled, the interactive TUI queries LM Studio and opens
an actor-model picker at startup. Reopen it at any time with:

```text
/act-model
```

An exact LM Studio model ID can also be selected directly:

```text
/act-model qwen2.5-coder-7b-instruct
```

Selection persists `enabled = true`, `require_registered_model = false`, the
external `model_id`, and `backend_model` to the user configuration. Codex then
reloads the configuration for the active session.

### 4. Run Codex

Start this repository's Codex normally:

```powershell
Set-Location E:\AI\Coding\codex
& $codex
```

Ask Codex to run tests, linting, a scanner, a build, or inspect logs. Eligible
large completed-command output is captured locally and summarized by LM Studio.
Small output and any local-analysis failure automatically remain on the raw
cloud path.

## Option B: Download a GGUF Through Codex

Set the storage path in `%USERPROFILE%\.codex\config.toml`:

```toml
[local_models.storage]
models_dir = "E:\\AI\\Models\\codex"
temp_dir = "E:\\AI\\Models\\codex\\.downloads"
max_disk_gb = 500

[local_models.analysis]
enabled = true
model_id = "local-log-analyst"
require_registered_model = true
min_output_bytes = 65536
max_input_bytes = 2097152
backend_url = "http://127.0.0.1:1234/v1"
backend_model = "local-log-analyst"
```

Search, inspect, and download a specific artifact:

```powershell
& $codex local-model search "qwen coder gguf"
& $codex local-model inspect <owner/repository> --revision main
& $codex local-model download <owner/repository> <model-file.gguf> `
    --model-id local-log-analyst `
    --revision main `
    --format gguf `
    --quantization Q4_K_M
```

For reproducibility, `download` resolves `main` to an immutable Hugging Face
commit before storing the artifact. Add `--sha256 <digest>` when the publisher
provides a trusted checksum.

Review and then run LM Studio activation:

```powershell
& $codex local-model activate-lm-studio local-log-analyst --dry-run
& $codex local-model activate-lm-studio local-log-analyst
& $codex local-model status
```

Activation copies the GGUF into LM Studio, preserves the Codex-managed source,
loads the exact artifact with GPU offload, starts a loopback server if required,
and verifies the configured model identifier.

## Useful Commands

```powershell
& $codex local-model path
& $codex local-model list
& $codex local-model list --json
& $codex local-model status
& $codex local-model remove <model-id>
```

## Troubleshooting

- **Configured model is unavailable:** compare `backend_model` exactly with the
  ID returned by `lms ps` or `/v1/models`.
- **The startup picker does not appear:** ensure local analysis is enabled and
  `backend_url` is configured, then use `/act-model` to retry.
- **Connection refused:** open LM Studio, load the model, and start the Local
  Server on port 1234.
- **Output is not routed locally:** confirm `enabled = true`; for an existing LM
  Studio model, confirm `require_registered_model = false`; then ensure command
  output exceeds `min_output_bytes`.
- **Model does not fit in VRAM:** load it manually in LM Studio with partial GPU
  offload or a smaller quantization. The detected RTX 3090 has 24 GiB VRAM.
- **Local analysis fails:** Codex fails open and gives the cloud model the
  original output. Raw artifacts remain under `<CODEX_HOME>/analysis-artifacts`.

Local evidence is treated as untrusted. It cannot authorize commands, edits, or
acceptance; the cloud-selected model remains the main reasoning brain.
