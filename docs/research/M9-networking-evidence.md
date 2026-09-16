# M9 Remote Networking Evidence

Status: Task 8 evidence checkpoint

Evidence branch: `m9-task8-network-evidence`

Verified implementation head: `99b168101e9fc31bdfbcea110bdb95d26978dac4`

Exact successful evidence run: GitHub Actions `35107158922`, job `104831250278`

This report records the Task 8 evidence required by `docs/plans/phase-1/M9-remote-networking.md`. It evaluates the existing Quinn baseline and the isolated Iroh candidate without changing Cross-Lab identity, session, policy, or production transport boundaries. It does not select the production remote-networking architecture; ADR-0009 remains responsible for that decision.

## Environment

The successful evidence run used the GitHub-hosted `ubuntu-24.04` runner image (`20260907.300.1`):

- Ubuntu 24.04.5 LTS
- Linux `6.17.0-1022-azure`
- `x86_64`
- 4 logical CPUs
- Intel Xeon Platinum 8573C runner CPU
- Microsoft full-virtualization environment
- approximately 15 GiB RAM; 3 GiB swap present and unused during environment capture
- `rustc 1.98.1 (48a229cea 2026-09-01)`
- `cargo 1.98.1 (797e8a9bc 2026-08-05)`

These are hosted-runner measurements, not dedicated hardware benchmarks. They are suitable for reproducibility and relative implementation evidence, but not for product performance guarantees.

## Dependency / feature tree

The evidence run executed `cargo tree -p crosslab-m9-networking -e features` before the full gate.

The experiment manifest keeps candidate networking dependencies inside `experiments/m9-networking`:

- Quinn baseline: `quinn = 0.11.11`
- Iroh candidate: `iroh = 1.2.0`, with explicit minimal feature selection
- owner relay fixture: `iroh-relay = 1.2.0`, with server/test support
- no `rust-libp2p` dependency is present

Iroh types and dependencies remain outside Cross-Lab domain crates and the production Quinn adapter.

## Security / owner-control eligibility

Task 8 preserved the M9 security constraints:

- The controlled relay is started and bound by the Cross-Lab experiment; the namespace evidence does not depend on public relay infrastructure.
- Iroh endpoints use explicit routing/relay configuration rather than a default public relay map.
- Persisted rendezvous metadata is restricted to `endpoint_id`, optional `relay_url`, and optional `ip`.
- Private keys, relay credentials/tokens, Cross-Lab credentials or proofs, exporter bytes, and benchmark payload contents are not represented in the rendezvous format or evidence output.
- Cross-Lab authentication completes before authenticated control or data evidence is accepted.
- Iroh-backed sessions remain `NetworkClass::Remote`; routing/path metadata does not grant identity, session, policy, or operation authority.
- A fresh connection requires fresh channel binding and Cross-Lab session authentication.

This is an experiment eligibility result, not production promotion. Iroh remains isolated until ADR-0009.

## Measurement method

The same-host command was run twice on the successful evidence job, including once under `/usr/bin/time -v`:

```text
cargo run -p crosslab-m9-networking --bin m9-networking -- local-all --samples 10 --payload-bytes 4194304
```

Each transport mode records ten samples of:

- protected connection establishment
- Cross-Lab session authentication
- control RTT
- fixed-size unidirectional bulk throughput
- shutdown

The TSV reproducibility header records Linux/x86_64, Rust `1.98.1`, 10 samples, 4 MiB payloads, Quinn `0.11.11`, and Iroh `1.2.0`. The primary successful run also recorded `rss_kib=68144` and `fd_count=12` at serialization time. Raw per-sample values are retained in Actions run `35107158922`; this report intentionally avoids treating hosted-runner timing noise as a stable product benchmark.

## Quinn baseline

The Quinn loopback baseline completed all ten samples with protected connect, Cross-Lab authentication, control RTT, fixed uni transfer, and shutdown evidence.

Observed behavior in the successful hosted run:

- protected connection establishment was in the low-millisecond range;
- Cross-Lab authentication dominated setup time relative to the raw protected connect;
- control RTT remained sub-millisecond to roughly millisecond scale;
- bulk throughput was the highest of the three measured modes in this run;
- shutdown remained bounded.

Quinn therefore remains a functioning comparison baseline for M9. Task 8 did not alter the verified M8 production Quinn adapter.

## Iroh direct

The direct Iroh mode completed all ten samples with the same Cross-Lab measurement surface.

Observed behavior in the successful hosted run:

- direct protected connect was materially slower than loopback Quinn but remained bounded;
- Cross-Lab authentication remained in the same broad order of magnitude as the Quinn-authenticated path;
- control RTT remained around millisecond scale;
- fixed-size bulk throughput was below the Quinn loopback baseline but above the owner-relay path in this run;
- shutdown remained bounded and short.

Direct connections use the ADR-0008 exporter profile after the full Iroh handshake; reconnect tests verify a new connection produces a new binding.

## Iroh relay

The owner-relay mode completed all ten samples and carried authenticated Cross-Lab control and data.

Observed behavior in the successful hosted run:

- relay protected connect had the highest setup latency and showed occasional hosted-runner outliers;
- Cross-Lab authentication remained bounded;
- control RTT was higher than same-host direct modes;
- fixed-size bulk throughput was lower than both loopback Quinn and Iroh direct in this run;
- shutdown remained bounded.

The relay path is still functionally important because it succeeds where the controlled endpoint-dependent NAT topology does not produce a direct path.

## Controlled NAT

The Linux namespace harness uses fixed owned state:

- namespaces: `cl-m9-ra`, `cl-m9-rb`, `cl-m9-a`, `cl-m9-b`
- bridge: `cl-m9-br`
- transit network: `172.30.90.0/24`
- peer A network: `10.90.1.0/24`
- peer B network: `10.90.2.0/24`
- owner relay: `172.30.90.1`

The script refuses pre-existing planned namespaces/bridge, confines forwarding/NAT setup to the owned namespace topology, uses nftables masquerade, blocks direct router-to-router UDP initially, later permits direct UDP, and cleans up owned processes/network state on exit.

The exact successful controlled-NAT output was:

```text
phase	relay_verified
network_class	remote
control_verified	true
data_verified	true
phase	direct_unavailable
direct_path_available	false
phase	reconnect_verified
binding_refreshed	true
session_refreshed	true
control_after_reconnect	true
```

Interpretation: owner-relay fallback, Cross-Lab authentication, control, and data succeeded. After direct UDP was enabled, Iroh `1.2.0` did not expose a direct IP path within the bounded observation window in this endpoint-dependent/symmetric NAT topology. The experiment records that as an observed path result rather than converting it into a session failure.

## Recovery / path changes

The controlled-NAT process test verifies recovery after the unavailable direct path:

- the relay-authenticated session remains usable through its evidence phase;
- a forced reconnect completes;
- the reconnect obtains a fresh channel binding;
- the reconnect obtains a fresh Cross-Lab session;
- authenticated control succeeds after reconnect.

The controlled namespace run did not produce a relay-to-direct transition, so it cannot itself prove same-connection continuity across that transition.

Separate deterministic Task 7/Task 8 tests cover the live relay-to-direct case and verify that one established connection keeps:

- `NetworkClass::Remote`;
- exporter binding;
- `SessionId`;
- control sequence state;
- active operation authority.

Those tests also verify that a new connection requires fresh authentication/authority rather than inheriting the old session.

## Resource observations

Linux report serialization records process RSS from `/proc/self/status` and open FD count from `/proc/self/fd`; unsupported platforms emit `-` rather than inventing a value.

The primary successful 10-sample / 4 MiB run recorded:

- report-time RSS: `68144 KiB`
- report-time FD count: `12`

The `/usr/bin/time -v` run provides an additional peak-RSS/process-cost observation in the raw CI log. These hosted-runner resource values are evidence of bounded execution, not a device budget or production SLO.

## Mobile / platform obligations

Task 8 evidence is Linux CI evidence only. Before any candidate becomes a production cross-device remote transport, Cross-Lab still needs platform-specific validation for at least:

- Android background/network lifecycle and Kotlin/Rust integration;
- iOS background execution, entitlements, network lifecycle, and Swift/Rust integration;
- real-device network transitions and sleep/wake behavior;
- Windows and macOS networking/firewall behavior;
- platform keystore/secure-enclave integration for production identity material.

No Task 8 result should be read as closing those obligations.

## Failures / anomalies

The evidence process exposed useful negative results rather than hiding them:

1. A finished outgoing uni-stream handle could previously drop before its driver first polled, making watch-channel closure look like explicit cancellation. A regression test now covers that scheduling/lifecycle race, and the driver retains its own cancellation sender for its lifetime.
2. The first direct-path failure regression tried to force the unavailable branch with a zero observation duration after a loopback IP path already existed. The deterministic seam was corrected so zero duration means no path observation.
3. The first controlled evidence workflow still asserted `direct_verified`; the topology actually completed and emitted `direct_unavailable` plus successful reconnect. The workflow expectation was corrected to record the observed NAT result.
4. Relay protected-connect timings showed hosted-runner outliers. They are retained as raw evidence rather than filtered from the run.
5. The controlled namespace topology does not establish a direct path after UDP is enabled. This is a real limitation of the tested topology/candidate combination, not a reason to weaken authentication or authority semantics.

## Decision matrix

| Criterion | Quinn baseline | Iroh direct | Iroh owner relay / controlled NAT | Evidence state |
| --- | --- | --- | --- | --- |
| Protected transport | Yes | Yes | Yes | Observed |
| Cross-Lab authentication before authority | Yes | Yes | Yes | Observed |
| ADR-0008-style channel binding | Existing verified profile | Verified | Verified | Observed |
| Bounded control/data | Yes | Yes | Yes | Observed |
| Owner-controlled relay | N/A | N/A | Yes | Observed |
| Remote classification preserved | N/A for local baseline | Yes | Yes | Observed |
| Same-connection relay-to-direct continuity | N/A | N/A | Deterministic tests pass; controlled symmetric-NAT run did not transition | Partially environment-dependent |
| Fresh binding/session on reconnect | Yes by existing Quinn semantics | Yes | Yes | Observed |
| Controlled endpoint-dependent NAT direct path | Not evaluated | Not the controlled topology | No direct path within bounded window; relay remained viable | Observed limitation |
| Linux reproducible benchmark/resource evidence | Yes | Yes | Yes | Observed |
| Mobile/device lifecycle evidence | Open | Open | Open | Not yet collected |
| Production selection | Existing local/LAN baseline | Not selected | Not selected | ADR-0009 pending |

## Libp2p trigger

**Libp2p trigger: no.**

Reason: the concrete Task 8 failure is that the fixed endpoint-dependent/symmetric NAT topology did not yield a direct path after direct UDP was permitted. The owner-relay path still authenticated and carried control/data, and forced reconnect refreshed binding/session correctly. This does not identify an Iroh-specific failed criterion that rust-libp2p/DCUtR would plausibly remove: endpoint-dependent/symmetric NAT is a general hole-punching limitation for which relay fallback remains necessary. Adding a second candidate would therefore add scope and maintenance cost without addressing the observed failure mode.

Task 9 is skipped. No `rust-libp2p` dependency should be added for M9 on the basis of this evidence.

## Task 8 evidence conclusion

Task 8 has enough evidence to proceed to ADR-0009 after branch integration:

- Quinn remains the verified local/LAN baseline.
- The isolated Iroh candidate preserves Cross-Lab auth/session/policy boundaries in deterministic and controlled relay tests.
- Owner-controlled relay fallback works in the controlled namespace topology.
- The fixed endpoint-dependent NAT topology does not produce a direct path within the bounded observation window; relay fallback remains required there.
- Reconnect creates fresh binding/session state and restores authenticated control.
- Task 9 is not triggered.
- Mobile/platform production obligations remain open and must be reflected in ADR-0009 rather than treated as already verified.
