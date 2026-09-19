# M10 Linux + Android Platform Evidence

This document records evidence for the M10 first real Linux + Android platform slice. CI evidence and real-device evidence are separate. **CI success does not satisfy the real-device lifecycle criterion.**

## Evidence status

- Task 9 integrated baseline: `main` at `f19aa4bb89d744f2ebf0f0fa2b204929fd8495a4`.
- Host CI evidence: pending Task 10 exact-head run.
- Linux real-device evidence: **pending**.
- Android real-device evidence: **pending**.
- Darkmatter/System-dark evidence: **blocked** until an authoritative Darkmatter palette is supplied.
- M10 completion: **open**.

## Safety rules for evidence

Do not record or attach:

- private keys, signing seeds, TLS private keys, credentials, authentication proofs, channel-binding bytes, provisioning-file contents, recovery material, tokens, passwords, or raw sensitive payloads;
- Android serial numbers, MAC addresses, local IP addresses, Wi-Fi SSIDs, usernames, home-directory paths, or other unnecessary device identifiers;
- full logs unless they have been reviewed and redacted.

Safe evidence should use commit/run identifiers, OS/build versions, architecture, application version, coarse device model, observed presentation state, and pass/fail results. When demonstrating reconnect, record only that the new session identifier differs from the old one; do not copy session identifiers into this document.

## Host CI evidence

The Task 10 CI gate must keep the existing Rust gate unchanged:

```text
cargo metadata --locked --no-deps --format-version 1
cargo audit
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Additional host-appropriate checks:

- default Linux desktop build;
- Linux desktop build with `development-provisioning`;
- deterministic UniFFI Kotlin generation;
- Android JVM unit tests;
- default Android debug assembly;
- Android debug assembly with `-PcrosslabDevelopmentProvisioning=true`.

These checks prove reproducible host builds only. They do not prove Android process lifecycle, radio/network transitions, or physical-device reconnect behavior.

## Real-device environment record

Fill this section only when physical-device evidence is collected.

| Field | Linux peer | Android peer |
| --- | --- | --- |
| Evidence ID | pending | pending |
| Cross-Lab commit | pending | pending |
| Build/application version | pending | pending |
| OS/version | pending | pending |
| Kernel/API level | pending | pending |
| Architecture/ABI | pending | pending |
| Desktop session / Android model | pending | pending |
| Development provisioning enabled | pending | pending |
| Evidence collector/date | pending | pending |

Use a coarse Android model name only. Do not record a device serial.

## Real-device procedure and results

Each scenario must be executed against the same Task 10 commit unless a row explicitly records a different commit. Record observable presentation state rather than secrets or raw protocol frames.

| Scenario | Required observation | Result | Evidence ID / notes |
| --- | --- | --- | --- |
| Initial authenticated connection | Both peers show trusted/connected/active state only after Cross-Lab authentication completes. | pending | pending |
| Android foreground -> background -> foreground | Background stops the mobile runtime; foreground creates one runtime and reconnects using fresh authentication. | pending | pending |
| Local network loss -> restoration | Both peers become disconnected without stale active-session authority; restoration yields a fresh authenticated session. | pending | pending |
| Desktop/Android disconnect -> reconnect | Reconnect succeeds with a new `SessionId`; record only `changed: yes`. | pending | pending |
| Revocation -> reconnect denial | Revocation closes/invalidates authority and a subsequent reconnect attempt is rejected. | pending | pending |
| Clean explicit shutdown | Runtime and transport shut down without a duplicate runtime or stale connected state. | pending | pending |
| Lifecycle churn | Repeated foreground/background and reconnect cycles do not create duplicate runtimes or duplicate active sessions. | pending | pending |
| Idle resource behavior | No busy polling; CPU/network activity remains bounded while idle. | pending | pending |
| Reconnect resource behavior | Reconnect attempts remain bounded and settle after success/failure without runaway background work. | pending | pending |
| Ayu Light selection | Canonical Ayu Light tokens render and remain selected as expected. | pending | pending |
| Darkmatter/System-dark | Run only after the authoritative Darkmatter palette is integrated. | blocked | authoritative palette absent |

## Suggested non-secret capture

For Linux, record only the output needed to establish the environment, for example distro/version, `uname -r`, and `uname -m`. For Android, record Android version/API level, ABI, application version, and coarse model. Review command output before copying it here.

For lifecycle/network scenarios, screenshots of the Cross-Lab status UI are preferred when they contain no secrets. If logs are needed, capture only presentation-safe state transitions and redact unrelated system/application data before attaching or summarizing them.

For idle/reconnect behavior, record the observation window and a coarse conclusion such as `idle CPU remained low; no repeated reconnect loop observed`. Do not claim numeric performance guarantees unless a repeatable measurement method and raw non-sensitive evidence are retained.

## Evidence acceptance criteria

Real-device evidence is acceptable only when:

1. the exact tested Cross-Lab commit is recorded;
2. both Linux and Android environment identifiers are present without sensitive identifiers;
3. every required lifecycle/network/security scenario above is pass/fail with an evidence reference;
4. reconnect evidence confirms a fresh session without publishing session values;
5. revocation evidence shows reconnect denial rather than only a generic network failure;
6. lifecycle churn shows no duplicate active runtime/session;
7. idle/reconnect behavior is bounded and does not depend on polling;
8. evidence has been reviewed for secrets;
9. Ayu Light is verified, while Darkmatter/System-dark remains explicitly open until its authoritative palette exists.

## Current blockers

Physical Linux + Android device execution is required to fill the pending rows. The repository/CI environment cannot manufacture this evidence. Darkmatter/System-dark also remains blocked by the missing authoritative palette.
