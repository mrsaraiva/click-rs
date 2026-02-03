# Gap Inventory (click-rs vs Python Click)

This document tracks **known behavior/API gaps** between `click-rs` and **Python Click 8.3.1**.
It’s meant to be living documentation: every intentional divergence should be recorded here,
and every unintentional divergence should be turned into a tracked task.

## Reference Baseline

- **Python Click:** `8.3.1` (pinned by `tests/parity/requirements.txt`)
- **Parity runner:** `tests/parity/run_parity.sh` creates a venv and installs the pinned Click wheel.
  - To compare against a local Click checkout, set `PARITY_ALLOW_CLICK_SRC=1` and `CLICK_SRC=/path/to/click/src`.

## Status Keys

- **Open:** known gap, not yet scheduled/implemented.
- **Planned:** work item exists in `docs/devel/ROADMAP.md`.
- **Done:** gap closed (or explicitly accepted as “by design”).
- **By Design:** intentional divergence (documented rationale).

## Inventory

| ID | Area | Gap | Impact | Status | Notes / Next Steps |
|----|------|-----|--------|--------|--------------------|
| GAP-TERMUI-001 | Terminal input | Hidden input (password prompts) is Unix-only; Windows falls back to visible input. | High (security/UX) | Done | Implemented Win32 console mode toggling (`ENABLE_ECHO_INPUT`) in `read_hidden_input` with fallback for non-console stdin. |
| GAP-TERMUI-002 | Terminal input | `getchar()` “raw” keypress is Unix-only; Windows requires Enter (line fallback). | Medium (UX) | Done | Implemented Win32 console key capture via `ReadConsoleInputW` (falls back to line-read when stdin isn’t a console). |
| GAP-TERMUI-003 | TTY detection | `isatty()` uses a mix of env heuristics + platform calls; may mis-detect in some environments. | Medium | Open | Consider switching to a dedicated crate (`is-terminal`) and keep env checks as a fast-path. |
| GAP-RUNNER-001 | Testing (`CliRunner`) | Capture is global-process state; concurrent `CliRunner::invoke*` calls are serialized via a global lock. | Medium | By Design | This matches the reality of redirecting process stdio; document as a contract for tests. |
| GAP-RUNNER-002 | Testing (`CliRunner`) | `charset` exists but is not used for decoding output. | Low | Open | Either implement decoding/normalization or remove the setting. |
| GAP-RUNNER-003 | Testing (`CliRunner`) | Non-`ClickError` failures are panics (rethrown) rather than “captured exceptions” like Click’s `catch_exceptions=True`. | Medium | Open | Consider a `catch_panics`/`catch_unwind` option that stores a synthetic exception message. |
| GAP-DECORATORS-001 | Decorators | `make_pass_decorator(ensure=...)` behavior is not implemented. | Medium | Planned | Needs a design decision: `Context` interior mutability or an alternate `ensure` API. |
| GAP-PARITY-001 | Parity coverage | Parity suites exist for phases 1–6; phases 7+ not yet represented as parity tests. | Low | Planned | Add parity for Windows-only terminal behavior as separate deterministic tests. |
| GAP-CI-001 | CI parity | CI runs `cargo test` on Linux/macOS/Windows; parity runner is not part of CI by default. | Low | Planned | Add an optional parity job (Linux) with pinned Click, or document as a local-only check. |

## Adding a New Gap

1. Add a new row with a unique `GAP-<AREA>-NNN` ID.
2. Include a minimal reproduction if possible (test name or parity phase/module).
3. If it’s intentional, mark **By Design** and write a short rationale in the Notes column.
