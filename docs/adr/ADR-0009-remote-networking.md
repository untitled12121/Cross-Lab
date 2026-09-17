# ADR-0009: Remote networking architecture

**Status:** Accepted  
**Date:** 2026-09-17  
**Accepted:** 2026-09-17

## Context

Cross-Lab requires remote connectivity across NATs and the public Internet without weakening the identity, trust, policy, logical-session, control-plane, or authorized-data-stream boundaries already proven through M8.

Quinn `0.11.11` is the verified local/LAN production baseline. M9 evaluated Iroh `1.2.0` as an isolated remote-connectivity candidate under `experiments/m9-networking`, with `iroh-relay 1.2.0` used for owner-controlled relay evidence. The candidate was kept outside Cross-Lab domain crates and production transport APIs.

The M9 evidence records the following:

- Iroh reproduces the accepted ADR-0008 `quic-tls-exporter-v1` profile after a full handshake.
- Existing Cross-Lab session authentication, control flow, operation-bound data streams, reconnect, revocation, cancellation, saturation, and shutdown semantics run over the candidate without an Iroh-specific identity or authorization path.
- Iroh-backed sessions remain `NetworkClass::Remote`.
- An owner-controlled relay carries authenticated Cross-Lab control and data without mandatory public relay infrastructure.
- A live relay-to-direct path change in deterministic tests preserves the exporter binding, `SessionId`, control sequence state, and active operation authority.
- A new Iroh connection requires a fresh channel binding, fresh Cross-Lab authentication, and fresh session authority.
- In the controlled endpoint-dependent/symmetric-NAT topology, relay fallback works but a direct IP path did not appear within the bounded observation window after direct UDP was enabled.
- Forced reconnect after that direct-path miss refreshes the binding and Cross-Lab session and restores authenticated control.
- The evidence report records `Libp2p trigger: no`; no M9 failure identified a concrete gap that rust-libp2p Relay v2/DCUtR/AutoNAT would plausibly remove.
- Real Android, iOS, Windows, macOS, sleep/wake, background-networking, firewall, entitlement, and secure-keystore validation remains open.

The Iroh experiment currently carries the repository's documented `paste 1.0.15` / `RUSTSEC-2024-0436` maintenance warning. M9 evidence is sufficient to select the architecture, but it is not a waiver for production dependency review.

## Decision

Cross-Lab selects the following remote-networking architecture:

1. **Quinn remains the local/LAN IP transport baseline.** M9 does not replace the verified M8 Quinn adapter.
2. **Iroh is the selected remote/NAT/relay connection substrate.** It is used beneath the existing Cross-Lab transport/session boundary and is not adopted as the Cross-Lab identity model, trust model, policy model, or universal connection fabric.
3. **Selection does not auto-promote the M9 experiment into production.** `experiments/m9-networking` and its Iroh dependencies are retained as evidence/regression material. A production Iroh adapter is introduced only by a consuming platform milestone after the remaining platform, lifecycle, key-storage, and dependency-review obligations are addressed.
4. **Cross-Lab identity remains independent from Iroh transport identity.** `DeviceId` and owner-authorized device credentials remain authoritative. Iroh `EndpointId`, Iroh `SecretKey`, relay URLs/tokens, path identifiers, and transport addresses are routing/infrastructure state only.
5. **Iroh connections reuse ADR-0008 exactly.** After the full QUIC/TLS handshake, an eligible implementation derives:
   - profile ID `quic-tls-exporter-v1`;
   - 32 bytes;
   - label `EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1`;
   - context `crosslab.quic.transport.v1`.
   No Iroh-specific identity transcript replaces Cross-Lab session authentication.
6. **0-RTT and early data do not carry Cross-Lab authority.** Ordinary session authentication begins only after a fully established protected connection can provide the accepted exporter binding.
7. **Every Iroh-backed Cross-Lab session is `NetworkClass::Remote` for its entire lifetime.** Relay/direct path changes are observability and performance state only; they never upgrade policy classification or bypass `LocalOnly`.
8. **A path change inside one live Iroh connection does not create new authority.** The existing exporter binding, Cross-Lab `SessionId`, sequence state, and active operation grants continue only while the same authenticated protected connection remains alive.
9. **A new Iroh connection is a fresh Cross-Lab session boundary.** Reconnect requires fresh exporter material, nonces/proofs, `SessionId`, negotiated state, and operation authority. Old proofs, sequence state, and grants are not reusable.
10. **Owner-controlled relay operation is a required deployment property.** Cross-Lab must support explicit owner-selected/self-hosted relay configuration and must not require an n0 relay, a Cross-Lab-operated public relay, or a Cross-Lab cloud account for normal remote connectivity.
11. **Direct-path availability is opportunistic, not guaranteed.** The controlled M9 topology proves that endpoint-dependent/symmetric NAT can remain relay-only even after direct UDP becomes available. Product behavior must treat relay fallback as a normal eligible route rather than claiming universal hole punching.
12. **rust-libp2p is not selected or added for M9.** Task 9 is skipped because the recorded evidence does not satisfy its trigger. A future requirement may independently re-evaluate libp2p through a new evidence-backed decision.
13. **M9 does not introduce seamless Quinn/Iroh session migration or adaptive route scoring.** Switching between the local Quinn adapter and a remote Iroh adapter remains a disconnect/new-session sequence until a later approved routing design exists.
14. **Production promotion is gated.** Before an Iroh adapter becomes a production cross-device transport, the consuming milestone must re-evaluate the Iroh dependency graph, including the `paste 1.0.15` / `RUSTSEC-2024-0436` warning, and must validate the relevant real-device platform lifecycle and secure key-storage behavior.

## Alternatives considered

### Quinn plus Cross-Lab-owned NAT traversal and relay

Rejected for M9. Building hole punching, reachability detection, relay protocol/server behavior, path discovery, and the associated operations/security tooling would add a large security-sensitive surface and duplicate mature networking work. The Iroh candidate already provides the required relay and path machinery while remaining isolated beneath Cross-Lab semantics.

### rust-libp2p remote fabric

Not selected and not prototyped in M9 because the evidence report records `Libp2p trigger: no`. The observed direct-path miss occurs in an endpoint-dependent/symmetric-NAT topology where relay fallback remains necessary in general; it does not identify an Iroh-specific failure that DCUtR or AutoNAT is expected to remove. In addition, the evaluated libp2p QUIC wrapper did not expose the already accepted TLS-exporter seam directly, increasing integration cost for no demonstrated M9 benefit.

### Iroh as the universal local and remote transport

Rejected. Quinn is already a narrow, verified local/LAN production adapter. Replacing it would expand M9 scope and erase a useful separation between proven local transport behavior and remote/NAT/relay behavior without evidence of a Cross-Lab benefit.

### Defer remote networking without selecting a candidate

Rejected. The M9 evidence establishes the required Cross-Lab authentication, owner-relay, lifecycle, path-observability, and bounded-behavior properties strongly enough to choose an architecture while explicitly carrying forward the real-device/platform obligations. Deferring the architecture would discard useful evidence without reducing those later validation obligations.

## Security impact

The decision preserves Cross-Lab identity and authorization above the remote transport. An Iroh relay may route encrypted traffic, but relay access does not grant Cross-Lab trust, session activation, capability permission, or operation authority.

Session proofs remain bound to the concrete protected connection through the exact ADR-0008 exporter profile and fresh authentication inputs. A captured proof from one connection is not valid on a fresh connection. Path changes within one live protected connection do not mint or widen authority.

Iroh `EndpointId` is a transport pseudonym, not a `DeviceId`. If a production deployment persists an Iroh transport key, that creates a stable routing identifier and therefore requires an explicit storage, rotation, and privacy policy. The transport key must remain separate from owner root, device-signing, administrative, and recovery key material.

An Iroh-backed session remains `NetworkClass::Remote` even when a direct path appears. This prevents route changes from silently satisfying `LocalOnly` or other locality-sensitive policy constraints.

Self-hosted relay support avoids making public infrastructure an authentication or availability root. Relays may still observe routing endpoint identifiers, connection timing, and traffic volume; payloads and Cross-Lab authentication material remain protected end to end.

No Cross-Lab private keys, authentication proofs, exporter bytes, relay shared tokens, or sensitive payloads may be logged or persisted by normal routing diagnostics.

The current transitive `paste 1.0.15` / `RUSTSEC-2024-0436` warning remains a production-promotion blocker until the dependency path is re-evaluated under repository security policy.

## Compatibility impact

This decision does not change Cross-Lab wire messages, protocol versions, identity credentials, canonical signing transcripts, pairing semantics, policy semantics, or logical-session public contracts.

`quic-tls-exporter-v1` remains compatibility-significant with the exact values accepted by ADR-0008. ADR-0009 clarifies that the profile describes required QUIC/TLS exporter semantics and may be implemented by another compatible QUIC/TLS stack; the Quinn-specific title of ADR-0008 does not make Quinn transport identity authoritative.

Existing Quinn sessions are unaffected. Iroh remote sessions use the same Cross-Lab authentication and authorization semantics above a different protected connection implementation.

M9 does not define live migration of a Cross-Lab logical session between Quinn and Iroh. A transport change across those adapters requires a fresh session.

## Operational impact

Cross-Lab will maintain two networking implementations when the remote adapter is promoted: Quinn for local/LAN and Iroh for remote/NAT/relay connectivity. The common boundary remains the existing Cross-Lab transport/session seam; candidate library types must not leak into domain crates.

Owner-hosted deployments may run an Iroh-compatible relay. Cross-Lab does not require a Cross-Lab-operated relay fleet or a mandatory vendor account. Relay capacity and availability must be treated as real operational dependencies because some NAT combinations remain relay-only.

The M9 experiment remains in the workspace as reproducible evidence and regression coverage until a later reviewed cleanup or promotion decision. Its presence does not authorize production use by itself.

M10 must validate at least Android background/network lifecycle, Kotlin/Rust integration, real-device network transitions, sleep/wake behavior, platform key storage, and the production dependency tree before remote networking is described as production-ready. iOS, Windows, and macOS obligations remain required when those platform slices consume the remote transport.

## Consequences

- M9 Task 9 remains skipped and `rust-libp2p` is not added.
- Quinn remains the verified local/LAN baseline.
- Iroh is the selected remote/NAT/relay architecture, while production promotion remains gated by the obligations above.
- The M9 Iroh experiment and dependencies are retained, but no production `transports/iroh` crate is created merely by accepting this ADR.
- A future production Iroh adapter must preserve the existing Cross-Lab `TransportConnection`/logical-session boundary and the security invariants in this ADR.
- The controlled symmetric-NAT direct-path limitation is an explicit architectural constraint; relay fallback remains required.
- The next platform milestone inherits explicit mobile lifecycle, secure-key-storage, and dependency-review obligations rather than assuming M9 Linux evidence covered them.
