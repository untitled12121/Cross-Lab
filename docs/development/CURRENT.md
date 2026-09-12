# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR: design committed for review; prototype implementation has not started.**

M1–M8 are complete and integrated into canonical `main`. M9 is isolated on `m9-remote-networking` from the verified M8 documentation head. M10 remains the first Linux + Android platform vertical slice after the remote-networking architecture is selected.

## Canonical Baseline

M8 — Quinn Transport is integrated and independently verified on `main`.

- final M8 feature head: `a93c6367be3174241cbab65b870f16f1543971f9`;
- feature-head CI: `34688453044` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and full workspace tests passed;
- PR #18 merged as `4a225976a16f34d7485fd259cacf767c84a60706`;
- post-merge main CI: `34688527098` — full required gate passed;
- canonical M8 documentation head: `ab602d62181b3dcba056874d0013e40522a68e8f`;
- documentation-head CI: `34688627614` — full required gate passed.

The M9 branch was created from exactly `ab602d62181b3dcba056874d0013e40522a68e8f`.

## M9 Research State

The Master Architecture requires M9 to prototype and measure Iroh and, where justified, rust-libp2p against the Quinn baseline, then select the remote NAT/relay architecture through an ADR.

Research completed before the M9 design:

- uploaded Iroh source inspected: package `iroh 1.2.0`, Rust 2024, `rust-version = 1.91`, `MIT OR Apache-2.0`;
- uploaded rust-libp2p source inspected: `libp2p 0.57.0`, Rust 2024, workspace `rust-version = 1.88.0`, MIT;
- current published versions were verified on 2026-09-12 as Iroh `1.2.0` and libp2p `0.57.0`;
- Iroh exposes TLS exporter material through `Connection::export_keying_material`, open/selected path observability through `paths`/`paths_stream`/`path_events`, explicit relay disable/default/custom modes, and a self-hostable relay server with infrastructure access controls;
- Iroh `presets::Minimal` avoids automatically adding n0 address lookup/default relay infrastructure, while the `N0` preset intentionally adds those services;
- rust-libp2p provides Relay v2, DCUtR, AutoNAT, and Swarm orchestration, but its current public QUIC connection wrapper keeps the underlying `quinn::Connection` private and exposes a broader `PeerId`/`Multiaddr`/Swarm model than Cross-Lab currently needs.

No Iroh or libp2p dependency has been added to a Cross-Lab production/domain crate.

## M9 Proposed Architecture

Design document:

- `docs/plans/phase-1/M9-remote-networking-design.md`;
- design commit: `9bed951d16aca0ceef08328162cb284ecc26f7d3` (`docs: add M9 remote networking design`).

The proposed direction is:

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

Key design invariants:

- Quinn remains the verified local/LAN baseline;
- Iroh is evaluated first as the isolated remote/NAT/relay candidate;
- rust-libp2p becomes a focused prototype only if Iroh fails a concrete criterion that libp2p plausibly solves;
- Iroh `EndpointId`/secret keys and relay credentials remain transport/infrastructure state and never become `DeviceId`, owner trust, or policy authority;
- required Iroh tests use `presets::Minimal` and explicit configuration, with no mandatory n0/Cross-Lab public service;
- self-hosted/owner-selected relay operation is a selection requirement;
- the Iroh candidate must reproduce ADR-0008 `quic-tls-exporter-v1` exactly after full handshake and reuse the existing Cross-Lab session-auth transcript;
- no 0-RTT/early-data authority is allowed;
- any session established through the Iroh remote adapter remains `NetworkClass::Remote` for its entire lifetime, even when the selected Iroh path upgrades from relay to direct;
- internal relay/direct path changes may affect metrics/diagnostics but not Cross-Lab identity, `SessionId`, authorization, or network-policy classification;
- a new Iroh protected connection is a full Cross-Lab reconnect with fresh binding, proofs, session state, and authority;
- no seamless migration between Quinn and Iroh is introduced in Phase 1;
- the evaluation harness is isolated under `experiments/m9-networking` and does not justify a new generic transport framework before evidence exists.

## M9 Evidence Required Before ADR Selection

The design requires deterministic tests without public infrastructure for exporter equality/freshness, existing session authentication, control/data semantics, reconnect/replay rejection, revocation, relay-only connectivity through a self-hosted relay, path-change observability, bounded saturation/cancellation, connection loss, and joined shutdown.

It also requires reproducible measurements against the same-host Quinn baseline for connection/session establishment, control latency, bulk stream throughput, relay-only behavior, relay-to-direct upgrade, recovery, resource usage, and shutdown. NAT traversal must be evaluated in a controlled translated-network topology where practical; loopback relay tests alone are not accepted as NAT evidence.

M9 may select the architecture with explicit real-device mobile lifecycle obligations for M10, but it must not claim Android/iOS background lifecycle verification that has not been performed.

## Intentional M9 Limits

M9 does not define a Cross-Lab cloud account, mandatory relay fleet, production relay operations, production Iroh-key persistence/rotation, discovery/pairing over Iroh, global route scoring, seamless cross-transport migration, TCP/TLS fallback, datagram/media APIs, UI, platform agents, privileged networking helpers, or a generic libp2p stack.

Production Iroh/libp2p dependency adoption happens only after the M9 ADR decision. Prototype dependencies, when approved, stay isolated in the experiment package until that decision.

## Exact Next Task

**Review and approve `docs/plans/phase-1/M9-remote-networking-design.md`.**

Per the architectural design workflow, no M9 prototype code or candidate dependency is added before the written spec is approved.

After approval:

1. write and commit the detailed M9 implementation/evaluation plan;
2. execute the plan in small RED/evidence → GREEN/measurement checkpoints;
3. preserve Quinn as the comparison baseline;
4. introduce Iroh only in the isolated experiment surface first;
5. trigger a libp2p prototype only if the documented decision rule requires it;
6. record measured evidence and select/reject the remote architecture through the M9 ADR;
7. reconcile the Master Architecture/CURRENT.md, verify the exact final head, merge to `main`, and verify canonical `main` before beginning M10.

## Resume Procedure

1. inspect canonical `main`, `m9-remote-networking`, any M9 PR, recent commits/workflows, and this file;
2. read the Master Architecture, M9 design, ADR-0008, M8 design, and transport/session contracts;
3. inspect relevant uploaded Iroh/rust-libp2p/Quinn source before changing related behavior;
4. do not start implementation until the written M9 design is approved;
5. after approval, follow the committed M9 plan and keep candidate dependencies isolated until the ADR permits production adoption;
6. work in small verifiable milestones and keep this file current;
7. merge only exact verified heads and verify canonical `main` after integration.
