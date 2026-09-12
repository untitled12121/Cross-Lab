# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR: Tasks 1–3 are GREEN; implementation is paused for manual review before Task 4.**

M1–M8 are complete on canonical `main`. M9 is isolated on `m9-remote-networking` in draft PR #19. M10 remains the first Linux + Android platform vertical slice after the remote-networking architecture is selected.

## Canonical Baseline

- M8 final feature head: `a93c6367be3174241cbab65b870f16f1543971f9`, CI `34688453044` — full gate passed.
- PR #18 merged as `4a225976a16f34d7485fd259cacf767c84a60706`.
- Post-merge `main` CI `34688527098` — full gate passed.
- Canonical M8 documentation head: `ab602d62181b3dcba056874d0013e40522a68e8f`, CI `34688627614` — full gate passed.
- `m9-remote-networking` was created from exactly `ab602d62181b3dcba056874d0013e40522a68e8f`.

## M9 Approved Architecture and Plan

Design: `docs/plans/phase-1/M9-remote-networking-design.md`.

Plan: `docs/plans/phase-1/M9-remote-networking.md`.

Durable plan checkpoint: `9add4d7e809d5579fed71b5b279ac9bc7481fc90`, CI `34691111935` — lockfile, rustfmt, workspace check, Clippy `-D warnings`, and full workspace tests passed.

Approved direction:

```text
Cross-Lab identity / trust / policy
             |
      LogicalSession
             |
     TransportConnection
        /            \
 local/LAN        remote/Internet
 Quinn M8          Iroh candidate
                      |
              direct path or relay
```

M9 invariants:

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` is evaluated first as an isolated remote/NAT/relay candidate; rust-libp2p remains conditional research.
- Iroh `EndpointId`, `SecretKey`, relay identity/credentials, addresses, and paths never become Cross-Lab identity/trust/policy authority.
- Required Iroh tests use `presets::Minimal`, explicit address/relay data, and no mandatory n0/Cross-Lab public infrastructure.
- Iroh must reproduce ADR-0008 `quic-tls-exporter-v1` exactly after a full handshake; no substitute binding is allowed silently.
- No 0-RTT authority.
- Every Iroh-backed Cross-Lab session remains `NetworkClass::Remote` for its whole lifetime, including relay/direct path changes.
- Path changes inside one protected connection do not change binding, `SessionId`, sequence state, policy classification, or operation authority.
- A new Iroh connection is a full Cross-Lab reconnect with fresh authentication and authorization state.
- Candidate dependencies stay under `experiments/m9-networking` until ADR-0009 selects a production architecture.

## Completed M9 Work

### Task 1 — Quinn baseline and typed metrics

Added non-publishable `experiments/m9-networking` with no Iroh/libp2p dependency at this checkpoint. It provides typed evaluation configuration/metrics, deterministic TSV output, and a raw Quinn `0.11.11` loopback baseline measuring protected connect, 32-byte control RTT, bounded unidirectional bulk throughput, and shutdown without weakening the production Quinn adapter boundary.

RED:

- head `6ce81462d3489a8b5855f39978a81224f841fa09`;
- CI `34692635013` failed check only on the intentionally missing baseline/config/metrics APIs.

GREEN:

- verified implementation head `785f93f914f454b1c902c76fe101c31d1db14e2a`;
- CI `34692865511` passed the full gate;
- documentation checkpoint `ea64385f04af076d41e263f0d2fd531ab7101f91`, CI `34692957283` passed.

### Task 2 — Direct Iroh and ADR-0008 exporter compatibility

Added Iroh `1.2.0` only to `experiments/m9-networking` with `default-features = false` and reviewed experiment features `portmapper`, `test-utils`, and `tls-ring`.

A temporary branch-only workflow generated the dependency lockfile because the connector execution environment did not provide a usable local Cargo checkout. The workflow committed `Cargo.lock` as `github-actions[bot]` at `cf515b9f2ae75cec9ce18291090d79167c539343` and was then removed at `320ba1691bb0f42edc912da5d7375f6cada2b7d0`. The bot has no runtime or architectural role in Cross-Lab.

Implemented:

- `candidate::binding` deriving the exact ADR-0008 profile: `quic-tls-exporter-v1`, 32 bytes, label `EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1`, context `crosslab.quic.transport.v1`;
- explicit Minimal Iroh loopback endpoints with relay disabled;
- direct connection using the server's explicit `EndpointAddr`, not transport identity alone;
- connection lifetime retained privately until Task 3 introduced a real experiment-only stream consumer;
- clean owned endpoint shutdown.

Deterministic evidence proves:

- both peers derive identical exporter bytes on one full Iroh connection;
- a fresh Iroh connection derives different exporter bytes;
- Minimal endpoints without explicit address data cannot connect merely from `EndpointId`;
- required tests use no public/n0 infrastructure.

RED:

- head `c62c683fa55333c568774611dceda5fe346e0c44`;
- CI `34693322026` passed lockfile/format and failed check only because the intended `candidate` implementation did not exist.

GREEN:

- verified implementation head `3f8aada0e6e3208664dc1d2efb2e25a98fb95752`;
- CI `34693556913` passed lockfile verification, rustfmt, workspace check, Clippy `-D warnings`, and full workspace tests;
- documentation checkpoint `d28cae010745feae038067ed3f83584809fbe495`.

### Task 3 — Bounded record framing and reserved Iroh control bridge

Added an experiment-local control transport surface over the fully established Iroh connection. No production/domain transport API was generalized and no new Cross-Lab domain error was introduced.

Implemented:

- `candidate::runtime::CandidateConfig` with independent typed limits matching all M8 `QuicTransportConfig::default()` semantic limits;
- private runtime terminal state using an atomic flag plus Tokio watch notification;
- owned `TaskRegistry` whose `close_and_take()` stops new task admission before returning all handles for joined shutdown;
- u32 big-endian record framing with size validation before body allocation, exact-limit acceptance, empty-record rejection, and truncated-body failure;
- one explicitly reserved Iroh bidirectional control stream, promoted only after a private framed `crosslab-m9-control-v1` marker is received;
- bounded Tokio mpsc outbound/inbound queues sized by `CandidateConfig`;
- synchronous nonblocking control APIs preserving existing `ControlSendError::{Full, TooLarge, Closed}` ownership semantics and `ControlReceiveError::{Empty, Closed}`;
- connection-close monitoring and terminal propagation;
- joined bridge shutdown followed by owned endpoint shutdown;
- experiment-only `DirectPair` Iroh connection accessors, added only when Task 3 became their first real consumer. No Iroh type was added to a production/domain public API.

RED:

- formatted RED head `78084e1bb1821468e8a4e22fa90df045de6a7469`;
- CI `34694277733` passed lockfile/format and failed check only on the intentionally absent `candidate::{control,record,runtime}` modules and connection accessors.

GREEN:

- implementation commit `9c58d20185322416de4a25dc2fe42b6a636710b2`;
- verified formatted head `8698db4561696a38e4bda7b35500ae3c8d8da663`;
- CI `34694599723` passed lockfile verification, rustfmt, workspace check, Clippy `-D warnings`, and the complete workspace test suite;
- all eight Task 3 tests passed: config-limit parity, exact-limit record round trip, hostile declared-length rejection before body allocation, truncated-body rejection, oversize ownership-preserving rejection, ordered bounded backpressure, peer-close propagation, and joined shutdown.

## Required Evidence Still Outstanding

M9 still must prove bounded uni-stream semantics, the experiment-only `TransportConnection` implementation, existing Cross-Lab session authentication over Iroh, lifecycle/reconnect/revocation behavior, owner-controlled relay-only operation, relay/direct path invariants, controlled NAT traversal/recovery evidence, comparable resource/performance measurements, and the ADR-0009 decision.

Loopback relay tests alone are not NAT evidence. Real Android/mobile lifecycle remains an explicit M10 obligation and is not claimed by M9.

## Review Hold and Exact Next Task

**Do not begin Task 4 until the current Tasks 1–3 implementation has been manually reviewed and approved.**

After approval, the exact next task is **Task 4 — bounded uni-stream bridge and experiment-only `TransportConnection`** from `docs/plans/phase-1/M9-remote-networking.md`.

Task 4 must cover ordered opening/chunks, exact limits, stream-slot and chunk-queue saturation, sender reset/drop -> `Cancelled`, receiver stop/cancel -> sender `Closed`, clean FIN -> `Finished`, connection loss, and joined shutdown. It may implement `TransportConnection` only inside the non-publishable experiment; no Iroh type may enter a production/domain public API.

## Repository Hygiene Note

Two empty temporary refs, `m9-task3-red-temp` and `m9-task3-work`, were accidentally created from Task 2 checkpoint `d28cae010745feae038067ed3f83584809fbe495` while preparing Task 3. No work was committed to them. They are not part of PR #19 and are safe to delete.

## Resume Procedure

1. verify `main`, `m9-remote-networking`, PR #19, recent commits/workflows, and this file;
2. read the Master Architecture, approved M9 design/plan, ADR-0008, and existing transport/session contracts;
3. inspect relevant Iroh/Quinn research/API behavior before related implementation;
4. preserve the current manual-review hold until explicitly approved;
5. after approval, execute each remaining task RED -> verified failure -> minimal GREEN -> full gate;
6. keep candidate dependencies isolated and `CURRENT.md` current with exact commits/CI;
7. trigger libp2p only if the plan's evidence rule is actually satisfied;
8. merge only an exact verified reviewed M9 head and verify canonical `main` afterward.
