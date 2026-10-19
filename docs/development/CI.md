# Change-Aware CI

Cross-Lab uses [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) for a **lightweight, path-aware default** and a **manual full verification** option. The protocol fuzz workflow remains independently scoped in [`fuzz.yml`](../../.github/workflows/fuzz.yml).

## Automatic behavior

- **Documentation-only pushes and PRs** (`docs/**`, `*.md`, `*.mdx`) do not start normal CI.
- **Android-only app changes:** Android unit tests. Shared Rust and native APK assembly are not rebuilt unnecessarily.
- **Linux desktop-only changes:** Rust formatting, workspace check, Clippy and desktop-focused tests. No Android SDK/build.
- **Shared Rust crates, transport, build scripts, design assets or workflow changes:** full Rust + Android platform checks, including relevant security verification. Cargo/dependency/security-related changes run `cargo audit` as a separate job.
- **Fuzz target-only changes:** existing path-scoped `Fuzz Smoke` workflow, not the full Rust/Android matrix.
- **Unknown or unreadable Git changes:** run everything (fail safely rather than silently skipping required validation).

The small `Determine checks` job validates the pure [scope classifier](../../.github/scripts/ci_scope.py), compares the actual Git event boundary and selects jobs. Unrelated jobs are marked skipped. A single `CI Gate` result fails if any **selected** check fails/cancels; intentionally skipped checks do not fail the gate.

For an open PR containing code, GitHub computes the **whole PR diff** against its base. A later documentation-only commit on that code PR may retrigger its existing code checks. This deliberately prevents treating previously unverified code as tested. Create separate documentation-only PRs when truly independent.

## Manual trigger

Open **GitHub → Actions → CI → Run workflow**, select the desired branch/ref and choose:

| Scope | What runs |
| --- | --- |
| `full` (default) | All formatting, dependency audit, complete Rust and Android verification including full builds and workspace tests |
| `rust` | Rust format, dependency audit, desktop builds and workspace tests (no Android job) |
| `android` | Android unit tests and both normal/development APK builds (no Rust desktop job) |
| `format` | Rust workspace formatting only |
| `audit` | Cargo dependency security audit only |

The manual `full` scope is appropriate before a release or a major cross-platform integration merge. If a branch changes a trust, privilege, protocol or permission boundary, run and review its applicable focused security regressions even when an automatic quick scope is sufficient for ordinary UI work.

## Job implementation

- Rust default quick check: locked metadata, Rust workspace check, FFI unsafe-boundary check, Clippy, and desktop crate tests. Full mode adds UniFFI generation, Linux desktop standard/development builds, workspace tests, tool/build/theme validation.
- Android default quick check: SDK bootstrap tests and Android debug unit tests (including compile). Full mode adds normal and development Android assemblies.
- Dependency audit is separate and runs for Cargo inputs and sensitive Rust policy/identity/protocol changes. The full/manual Rust modes always audit.
- `concurrency.cancel-in-progress` cancels obsolete runs for the same PR/ref so earlier commits do not burn runner time.

No workflow uses `[skip ci]` as a substitute for choosing an appropriate verification profile. No passed status implies physical Linux/Android device acceptance.

## Required status configuration

The repository's branch protection rules may need to use the stable **`CI Gate`** context instead of requiring every platform job. Skipped jobs are expected for unrelated changes. **Docs-only changes have no normal CI run**, so a required CI check configured indiscriminately on *every* documentation-only PR could remain pending. Review repository rulesets/required checks when enabling this workflow; branch protection cannot be read with the current GitHub connector permissions. Do not disable security checks merely to allow merges.

## Maintenance

To edit scope logic, change [`ci_scope.py`](../../.github/scripts/ci_scope.py) and its [regression tests](../../.github/scripts/test_ci_scope.py) together. Unknown inputs must choose full validation. Add new platform directories explicitly rather than accidentally skipping them. Changes to the CI workflow and its classifier deliberately exercise full CI once, to validate the routing rules themselves.
