# M9 Foundation Security Hardening Assessment

**Status:** Verified  
**Scope:** Foundation findings discovered while reviewing M9 Tasks 1–3, before any Task 4 implementation.

## Result

The M9 foundation hardening gate is complete. The changes enforce existing Cross-Lab architecture and security invariants; they do not introduce a new architecture decision.

The verified implementation head before this assessment was `26b9ecd8abf8f7386e440bc4862a6e259d1654fe`.

## Fixed Findings

### Authorization provenance

- `SimNode` receives an explicit `NetworkClass`; no implicit local-network authority remains in its constructor path.
- Inbound control validates that the supplied local trust record belongs to the authenticated owner/device before sequence or policy dispatch.
- Pending or revoked peer trust fails closed, and a trust-revision change is rejected against the authenticated session snapshot.
- Regression coverage proves `NetworkClass::Remote` cannot satisfy `Constraint::LocalOnly`, local revocation is fatal before explicit teardown, and a trust record for another device is a fatal provenance mismatch.

### Bounded operation-state reclamation

- `StreamAdmission` reclaims terminal operation registrations once they can no longer authorize work.
- Failed admission that makes an operation terminal also releases registration capacity.
- `cancel_all` cancels and releases registered operation state.
- A completed `SingleStream` operation is retired; later reuse therefore returns `OperationNotFound`, not a duplicate-index error.
- Capacity-reuse regressions cover completion, expiry, and cancellation.

### Candidate task ownership

- The experiment-local task registry closes task admission under the same mutex used for spawn registration.
- A future offered after shutdown admission closes is dropped rather than detached.
- Shutdown takes and joins all registered task handles.
- Deterministic regression coverage verifies post-close spawn rejection.

### Transport metadata privacy

- `ConnectionMetadata` uses a custom `Debug` implementation that redacts local and remote endpoint descriptions while preserving non-sensitive metered state.
- Regression coverage verifies endpoint strings do not appear in ordinary debug output.

### Supply-chain and CI reproducibility

- CI runs on `ubuntu-24.04` and pins `actions/checkout` to commit `3d3c42e5aac5ba805825da76410c181273ba90b1`.
- Rust remains pinned to `1.98.1`.
- CI installs pinned `cargo-audit 0.22.2` and requires `cargo audit` to succeed.
- The fuzz workflow uses dated `nightly-2026-09-12`, pinned `cargo-fuzz 0.13.2`, the pinned checkout commit, and a committed `fuzz/Cargo.lock` verified with `--locked`.

## Verification Evidence

Exact code head `26b9ecd8abf8f7386e440bc4862a6e259d1654fe` passed:

- Rust CI run `34724799407`: lockfile verification, dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and the complete workspace test suite.
- Fuzz Smoke run `34724799451`: fuzz lockfile verification, fuzz formatting, and all five bounded fuzz targets.
- Focused stale-test reconciliation run `34724713810`: `s007_authorized_single_stream_flows_in_order_and_cannot_be_reused` passed after aligning its expected terminal-authority result with the core contract.

## Dependency Audit Note

`cargo audit` reported **no vulnerability failure**. It emitted one allowed maintenance warning:

- `paste 1.0.15`, `RUSTSEC-2024-0436` — unmaintained.

The dependency is not direct Cross-Lab production authority code. `cargo tree -i paste` traced it through the isolated M9 Iroh candidate (`netlink-packet-core` / `netlink-packet-route` -> `netwatch` and related Iroh networking paths -> `iroh 1.2.0` -> `crosslab-m9-networking`). The warning is not suppressed. Re-evaluate it when updating the Iroh candidate and before ADR-0009 promotes any candidate architecture.

## Deferred / External Items

- `main` branch protection is currently disabled. This is a repository-administration control, not an in-tree code change. Enable protection with force-push/deletion disabled and require the Rust CI check when the necessary GitHub administration capability is available.
- M9 still needs Task 4+ evidence for bounded Iroh uni-stream semantics, Cross-Lab session/lifecycle behavior over Iroh, owner-controlled relay operation, path invariants, controlled NAT evidence, comparative measurements, and ADR-0009.
- Real mobile lifecycle evidence remains an M10 obligation.

## Gate

The foundation hardening gate is satisfied. The separate manual-review hold recorded in `docs/development/CURRENT.md` remains in force before M9 Task 4 begins.
