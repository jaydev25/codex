# Local Actor Failure Ledger

> Append-only evidence for improving local actor reliability. Never include
> source content, prompts, secrets, artifact paths, or full model responses.

## Recording rules

Record one row for every actor or orchestration failure that prevents a
proposal from reaching reviewed execution. Keep validator messages concise and
bounded. Use these classes:

- `patch_framing` — malformed raw `apply_patch` syntax.
- `unauthorized_operation` — output requested a tool or path not authorized by
  the assignment.
- `schema` — actor output did not match the strict response schema.
- `transport` — the loopback backend could not complete the request.
- `orchestration` — Codex lost, conflicted with, or could not recover otherwise
  valid task state.
- `edit_validation` — a structured edit had stale, missing, truncated, or
  ambiguous context.
- `test_failure` — a reviewed proposal executed but failed acceptance tests.

`Applied` must be `no` unless tool evidence confirms the proposal was applied.

## Failures

| Date | Task ID | Stage | Class | Bounded reason | Action | Applied |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-09-26 | `arch-011-actor-usage-reporting` | Proposal validation | `patch_framing` | Comment appeared where an Add/Update/Delete hunk header was required | Replanned with one-file patch instructions | no |
| 2026-09-26 | `arch-011-actor-usage-core-v2` | Orchestration return | `orchestration` | First revised call completed without surfacing a reviewable proposal; retry then reported changed/running/escalated task state | Replanned under a new task ID | no |
| 2026-09-26 | `arch-011-usage-parse-v3` | Proposal validation | `patch_framing` | Patch string did not begin with `*** Begin Patch` | Narrowed to one file with explicit framing | no |
| 2026-09-26 | `arch-011-usage-parser-v4` | Authorization validation | `unauthorized_operation` | Actor proposed a test command without planner-approved `exec_command` access | Added the missing tool authorization under a new task ID | no |
| 2026-09-26 | `arch-011-usage-parser-v5` | Proposal validation | `patch_framing` | Raw Rust source appeared where an Update hunk header was required | Completed the tracked stage through cloud-owned implementation and validation | no |
| 2026-09-26 | `arch-013-structured-edits-model` | Proposal validation | `patch_framing` | Patch string did not begin with `*** Begin Patch` | Began replacing actor-authored patch strings with structured edits rendered by trusted code | no |
| 2026-09-27 | `arch016-lmstudio-runtime-1` | Assignment validation | `orchestration` | Planner-supplied reference context exceeded the actor context bound before inference | Replanned with smaller truncated reference excerpts under a new task ID | no |
| 2026-09-27 | `arch016-lmstudio-runtime-2` | Cloud proposal review | `orchestration` | Proposal assumed unavailable dependencies, parsed fields absent from real LM Studio JSON, and used environment-dependent tests instead of exercising production validation | Returned bounded review feedback for a second actor attempt | no |
| 2026-09-27 | `arch016-lmstudio-runtime-2` | Cloud proposal review, attempt 2 | `orchestration` | Revised proposal retained an unavailable dependency, compared the selected identifier to a placeholder, and contained a malformed load-command call | Returned exact compile and behavior feedback for the final actor attempt | no |
| 2026-09-27 | `arch016-lmstudio-runtime-2` | Cloud proposal review, attempt 3 | `orchestration` | Final proposal reintroduced an unavailable dependency, mismatched camelCase LM Studio fields, ignored load failure status, and contained non-asserting tests | Ended the three-attempt assignment and split metadata logic from process execution under a new task ID | no |
| 2026-09-27 | `arch016-lmstudio-metadata-v2` | Cloud proposal review | `orchestration` | Metadata proposal omitted modelKey renames, skipped public JSON parsing, did not enforce the known maximum, and attached the sibling test module recursively | Returned exact corrections for the second actor attempt | no |
| 2026-09-27 | `arch016-lmstudio-metadata-v2` | Cloud proposal review, attempt 2 | `orchestration` | Revised DTOs still lacked camelCase renames, returned a boxed dynamic error across a future blocking boundary, and used partial assertions contrary to acceptance criteria | Returned final exact corrections | no |
| 2026-09-27 | `arch016-lmstudio-metadata-v2` | Cloud proposal review, attempt 3 | `test_failure` | Final proposal's fallback test expected a maximum absent from its fixture and the implementation retained unused imports and partial-object assertions | Closed the assignment and split production code from test authoring under new task IDs | no |
| 2026-09-27 | `arch016-context-parser-micro` | Cloud proposal review | `orchestration` | Pure parser logic was acceptable, but the sibling test module declared itself recursively instead of being declared by the implementation file | Requested a layout-only repair with unchanged behavior | no |
| 2026-09-27 | `arch016-context-parser-micro` | Cloud proposal review, attempt 2 | `edit_validation` | Corrected proposal emitted two separate add operations for the same implementation path, making the second execution-plan entry impossible to apply | Requested one consolidated add per file | no |
| 2026-09-27 | `arch016-lmstudio-helper-tests-micro` | Actor transport | `transport` | Loopback `/v1/chat/completions` request failed before the actor returned a proposal | Escalated immediately and checked LM Studio model/server state | no |
| 2026-09-28 | `arch016-loaded-model-identifiers-tests-v1` | Assignment validation | `orchestration` | Planner submitted actor schema version 1 after the installed protocol had moved to version 2 | Retried under a new immutable task ID with schema version 2 | no |
| 2026-09-28 | `arch016-loaded-model-identifiers-tests-v1` | Retry orchestration | `orchestration` | Reusing the rejected task ID with a changed assignment correctly triggered the immutable-assignment guard | Started the corrected assignment under `arch016-loaded-model-identifiers-tests-v2` | no |
| 2026-09-28 | `arch016-loaded-model-identifiers-tests-v2` | Cloud proposal review | `orchestration` | Attempt 1 reversed the helper arguments and proposed direct `cargo test` contrary to repository policy and acceptance criteria | Returned both exact issues through bounded failure feedback | no |
| 2026-09-28 | `arch017-usage-hud-formatter-v1` | Cloud proposal review | `orchestration` | Attempt 1 used assertions instead of snapshots, reversed the sibling-module declaration, widened visibility, used an incompatible token type, and proposed the full test suite | Returned exact layout, API, snapshot, and command feedback | no |
| 2026-09-28 | `arch017-usage-hud-formatter-v1` | Cloud proposal review, attempt 2 | `orchestration` | Proposal still reversed the sibling-module declaration, used the wrong formatter import and unchecked cast, combined snapshot states, and retained an unwanted header | Returned exact source-layout and snapshot corrections | no |
| 2026-09-28 | `arch017-usage-hud-formatter-v1` | Cloud proposal review, attempt 3 | `orchestration` | Proposal omitted HUD separators, retained the unwanted header, and constructed a manual style instead of using the TUI styling helper | Closed the three-attempt assignment and materially replanned the interface | no |
| 2026-09-28 | `arch017-usage-hud-formatter-v2` | Cloud proposal review | `orchestration` | Revised attempt 1 copied negative acceptance wording into a source comment and omitted inline expected snapshots | Returned positive-output-only feedback | no |
| 2026-09-28 | `arch017-usage-hud-formatter-v2` | Cloud proposal review, attempt 2 | `edit_validation` | Proposal emitted two add edits for the same new path and used the wrong test import path | Required one complete add per new path | no |
| 2026-09-28 | `arch017-usage-hud-formatter-v2` | Cloud proposal review, attempt 3 | `orchestration` | Proposal again copied negative wording and omitted the required sibling test-module declaration | Closed the assignment and split implementation from tests | no |
| 2026-09-28 | `arch017-usage-hud-function-v3` | Cloud proposal review | `orchestration` | Formatter-only attempt 1 used positional format arguments contrary to repository style | Returned a three-expression style-only repair request | no |
| 2026-09-28 | `arch017-refine-actor-planner-v1` | Cloud proposal review | `orchestration` | Planner-guidance attempt 1 omitted the explicit one-complete-add-per-new-path rule derived from repeated actor failures | Returned the single missing guidance requirement | no |

## Improvement signals

Current evidence shows that multi-file new-module tasks amplify duplicate-add,
module-layout, and copied-negative-wording failures. Prefer one file or one
coherent responsibility per microtask, one complete add per new path, exact
function signatures, positive-output acceptance criteria, bounded source
context, and focused repository-approved test commands. Split implementation
from tests after repeated framing or layout confusion. Do not weaken path,
authorization, schema, or patch-parser validation to improve apparent success.
