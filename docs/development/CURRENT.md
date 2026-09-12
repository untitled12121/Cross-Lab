# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M8 — Quinn Transport: Tasks 1–3 are GREEN; Task 4 bounded control bridge is next.**

M1–M7 are complete and integrated into canonical `main`. M8 remains isolated on `m8-quinn-transport` and is not yet ready to merge.

## Canonical Main State

- Canonical `main` documentation head before M8: `b2e16f27a5c6d8307c58f4c6a4c760bb867ff223` (`docs: record M7 integration`).
- Post-M7 canonical CI: `34681661930` — passed.
- M8 branch was created from that exact verified head.

## M8 Planning and Architecture

Approved/committed planning artifacts:

- `docs/plans/phase-1/M8-quinn-transport-design.md`;
- `docs/adr/ADR-0008-quinn-channel-binding-profile-v1.md`;
- `docs/plans/phase-1/M8-quinn-transport.md`.

M8 preserves these boundaries:

- `crosslab-core` remains runtime/transport implementation neutral;
- Quinn, Tokio, rustls, socket, endpoint, and TLS certificate types stay inside `transports/quic`;
- Quinn/TLS provides confidentiality/integrity and channel-binding material, not Cross-Lab device identity authority;
- the accepted binding profile is `quic-tls-exporter-v1`, 32 bytes, label `EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1`, context `crosslab.quic.transport.v1`;
- no `Connecting::into_0rtt` or 0-RTT authority is allowed;
- reconnect means a fresh protected connection and therefore fresh binding/session/authentication/authorization state;
- all application queues, record sizes, stream concurrency, windows, cancellation, and shutdown behavior remain explicitly bounded;
- no discovery, NAT traversal, relay, Iroh, libp2p, datagram consumer, route migration, persistence, UI/platform, privileged-service, or production certificate-provisioning scope enters M8.

## M8 Dependency and Research State

Production dependency baseline:

- Quinn `0.11.11`, pinned with only `runtime-tokio` + `rustls-ring`;
- Tokio `1.53.1`;
- rustls `0.23.44` where concrete loopback trust construction requires it;
- rcgen `0.14.10` dev-only for ephemeral loopback certificate fixtures.

The uploaded Quinn repository has now been successfully inspected. It is Quinn `main` at package version `0.12.0`; it is research/reference material only. Production remains pinned to the latest separately verified published stable Quinn `0.11.11`. Relevant stream and connection behavior was reconciled against that uploaded source before implementation; architecture was not copied blindly.

## M8 Completed Work

### Task 1 — Transport-neutral outbound bounds

Implemented:

- ownership-preserving `ControlSendError::TooLarge(Vec<u8>)`;
- ownership-preserving `StreamOpenError::TooLarge(Vec<u8>)`;
- configurable memory-transport control/opening-frame limits;
- oversize rejection before queue/stream capacity is consumed;
- simulator stream runtime treats oversize as a local rejection rather than transport loss.

GREEN checkpoint:

- head `691aa11f2c7a101a1cb65e1999e6da4a1fee2f4d`;
- CI `34684023292` — lockfile, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.

### Task 2 — Quinn crate and TLS-exporter binding

Implemented:

- workspace member `crosslab-transport-quic` under `transports/quic`;
- exact dependency pins and generated lockfile;
- private Quinn TLS-exporter binding derivation implementing ADR-0008;
- real loopback Quinn/TLS test using an explicitly trusted ephemeral rcgen certificate;
- proof that both peers derive identical binding bytes for one connection;
- proof that a fresh reconnect derives different binding bytes.

GREEN checkpoint:

- head `a032189481fee095bdf683535b982972188469c1`;
- CI `34684420001` — lockfile, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.

`QuicTransportConfig` was not introduced in this binding-only slice because it had no production consumer yet. It remains required by the approved plan and must enter with the first connection bridge slice rather than being omitted.

### Task 3 — Bounded private record framing

Implemented private transport framing:

```text
u32 big-endian declared length
[declared] record bytes
```

Properties now proven/implemented:

- exact configured record limit succeeds;
- declared oversize is rejected before body allocation;
- forbidden zero-length records are rejected;
- truncated bodies map to a bounded read failure;
- writes reject empty and oversized payloads before sending;
- no attacker-controlled `read_to_end` is used.

During GREEN verification, the first loopback test helper deadlocked because Quinn bidirectional streams are lazy: the peer cannot `accept_bi()` until the opener writes data. The uploaded Quinn source explicitly documents this behavior. The helper was corrected to write first and accept afterward; production framing code was unchanged.

GREEN checkpoint:

- current branch head `fa1d35ca370b5ca75d6c55b41c21a56b4a0f2b75`;
- CI `34684781173` — lockfile, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.

## Active Pull Request

- Draft PR #18: `feat: implement M8 Quinn transport`.
- Base: `main` at the original verified M8 baseline.
- Head branch: `m8-quinn-transport`.
- Do not merge until the entire M8 milestone is complete, reviewed, and the exact final head passes all required gates.

## Remaining M8 Work

The approved implementation plan still requires:

1. Task 4 — typed `QuicTransportConfig`, reserved control-stream bridge, bounded Tokio control queues, terminal connection state, connection-owned task shutdown/join semantics;
2. Task 5 — bounded authorized unidirectional stream bridge with opening-frame/chunk limits, saturation, FIN/reset/stop/cancellation semantics;
3. Task 6 — prove the existing Cross-Lab session-auth protocol over the Quinn exporter binding without creating a second authentication architecture;
4. remaining reconnect/revocation/failure/shutdown integration scenarios from the M8 plan;
5. final dependency/boundary/security review and full workspace verification;
6. update architecture/docs for completed M8 factual state;
7. only then merge the exact verified M8 head to `main` and verify canonical `main` after merge.

Temporary `#[cfg_attr(not(test), allow(dead_code))]` on the private binding/record modules is allowed only while those helpers have no production connection consumer. Remove those allowances when Task 4 wires the real connection bridge.

## Exact Next Task

Start **Task 4 RED** from `docs/plans/phase-1/M8-quinn-transport.md`.

Write real loopback tests first for the reserved control bridge proving at minimum:

- ordered control delivery;
- bounded local outbound queue saturation with ownership preservation;
- configured oversize rejection before queueing;
- remote connection close becomes terminal;
- explicit adapter shutdown closes the connection and joins all connection-owned tasks.

The RED run must fail because `QuicTransportConnection`/its required config/bridge behavior does not yet exist. Only after that verified failure should the minimal bounded implementation be added.

## Resume Procedure

1. inspect canonical `main`, `m8-quinn-transport`, PR #18, recent commits/workflows, and this file;
2. read the Master Architecture, M8 design, ADR-0008, active implementation plan, and relevant transport/session contracts;
3. inspect relevant existing Cross-Lab code and uploaded research source before implementing related behavior;
4. execute small RED → verified failure → GREEN milestones;
5. after meaningful progress, run the full gate, commit/push, and update this file;
6. do not rely on chat history or stash as the only copy of valuable work;
7. merge only the exact verified completed M8 head and verify canonical `main` after merge.
