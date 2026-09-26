# Phase 2 Clipboard Vertical Slice

**Status:** Active
**Date:** 2026-09-24
**Base:** PR #56 merged as d707b330b5d1008e8020267faa3755ad84920591

## Goal

Deliver the first real product capability on the authenticated-session and exact-policy foundation without weakening Cross-Lab's capability, privacy, reconnect, or platform boundaries.

The first product clipboard slice is text-only. Files/images remain part of the later resumable file-transfer work.

## Existing Baseline

The architecture already reserves:

- clipboard.read with protected operation get;
- clipboard.write with protected operation set;
- capability version 1.0 in Phase 1 fixtures;
- exact device-scoped default-deny authorization;
- opaque capability request/response bodies on the authenticated control channel.

Android clipboard reads are platform-restricted when the app is not focused/default IME, so Phase 2 must not depend on background clipboard monitoring.

## Task 1 — Event-driven product control pump — implemented

- Add a bounded Quinn control-readiness signal without changing identity or authorization semantics.
- Let RuntimeActor consume inbound control frames when signalled instead of polling.
- Publish bounded non-status NodeEvent values to the owning agent.
- Mirror only the existing runtime send operations needed by product capabilities.
- Keep reconnect, policy replacement, revocation, and transport-close behavior fail-closed.

Implemented on PR #57:

- Quinn exposes a bounded watch-based inbound-control readiness signal after a frame enters the existing bounded control queue;
- RuntimeActor drains control frames only on readiness, publishes bounded non-status NodeEvent values, and preserves reconnect/policy/revocation shutdown semantics;
- the trusted presence coordinator consumes the actor event stream so capability traffic cannot fill an unobserved queue;
- actor send wrappers expose only the existing capability advertisement/request/response paths;
- Quinn and runtime tests cover wake-driven delivery without introducing a polling loop.

This task does not define new wire semantics and can land before the clipboard profile is accepted.

## Task 2 — Clipboard profile decision — accepted

ADR-0018 defines the accepted product clipboard v1 compatibility surface:

- explicit request/response only for the MVP;
- clipboard.write/set: bounded UTF-8 text request, empty success body;
- clipboard.read/get: empty request, bounded UTF-8 text success body;
- no clipboard.changed auto-sync event in v1;
- no file/image clipboard payloads;
- no plaintext clipboard content in logs/audit/debug formatting.

ADR-0018 was accepted by the owner on 2026-09-25; implementation may now depend on this compatibility surface.

## Task 3 — Owner policy persistence — active

The first editable product permission requires durable policy state. Design this as a separate reviewed policy-store boundary rather than embedding rules into identity state or silently reusing identity-store semantics.

Requirements:

- exact device/capability/operation rules only;
- monotonic currentness and rollback/mixed-state rejection;
- Android protected currentness material and Linux protected currentness material;
- no clipboard payloads in the policy store;
- policy must be loaded before the presence/runtime session is created.

ADR-0019 was accepted by the owner on 2026-09-25. PR #58 merged the shared rollback/currentness-aware policy-store core plus Linux policy-state/Secret Service and Android AtomicFile/Keystore protected-currentness adapters after exact-head CI and Fuzz Smoke passed. Both production presence paths load validated policy before trusted-session runtime creation. The active `phase2-permission-edits` milestone wires exact owner edits through persist-before-apply storage commits, active-policy replacement acknowledgement, and fail-closed recovery before clipboard execution is added.

## Task 4 — Shared clipboard capability runtime

After the clipboard profile is accepted:

- advertise only runtime-available clipboard capabilities;
- require negotiated version plus exact policy before dispatch;
- bound and validate UTF-8 before platform delivery;
- correlate outbound requests with bounded session-local request state;
- cancel pending clipboard work on reconnect, policy revision, disconnect, revocation, or shutdown;
- expose ephemeral clipboard operations, not retained clipboard history.

## Task 5 — Linux and Android adapters

Linux:

- use GPUI's native clipboard access instead of adding a clipboard dependency;
- keep OS clipboard reads/writes in the desktop adapter/UI boundary.

Android:

- use ClipboardManager;
- read only from an explicit foreground user action when platform access is available;
- write received text with the platform API;
- mark sensitive local clipboard content where the platform contract supports it;
- never claim background clipboard monitoring.

## Task 6 — Product UI

Expose the smallest clear controls:

- current per-device clipboard permissions;
- explicit Send clipboard and Fetch clipboard actions when supported/allowed;
- actionable unavailable/denied/oversized/failure state;
- no plaintext clipboard preview/history in the control center.

## Verification

Infrastructure milestone head `956aac69b152a6a402d8952f6ec6900b041b237f` passed full Rust + Android CI `35970744991`.

- Rust format/check/clippy/test workspace gate;
- Quinn/runtime actor event-driven control tests;
- clipboard codec boundary/privacy tests after ADR acceptance;
- Linux desktop build + development build;
- Android JVM tests + debug/development assemblies;
- reconnect/policy-change tests prove pending clipboard authority is not carried across sessions;
- no plaintext clipboard content in Debug/error/log output.
