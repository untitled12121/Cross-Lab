<div align="center">

# Cross-Lab

**A local-first, owner-controlled cross-device ecosystem.**

Linux · Windows · macOS · Android · iOS / iPadOS

[![Stars](https://img.shields.io/github/stars/untitled12121/Cross-Lab?style=for-the-badge&labelColor=24283b&color=7aa2f7)](https://github.com/untitled12121/Cross-Lab/stargazers)
[![Issues](https://img.shields.io/github/issues/untitled12121/Cross-Lab?style=for-the-badge&labelColor=24283b&color=e0af68)](https://github.com/untitled12121/Cross-Lab/issues)
[![Contributors](https://img.shields.io/github/contributors/untitled12121/Cross-Lab?style=for-the-badge&labelColor=24283b&color=9ece6a)](https://github.com/untitled12121/Cross-Lab/graphs/contributors)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-7dcfff?style=for-the-badge&labelColor=24283b)](./LICENSE-MIT)

</div>

Cross-Lab is an open-source ecosystem for connecting devices owned by the same person across desktop and mobile platforms. It is designed to let those devices discover each other, establish cryptographic trust, negotiate capabilities, authorize operations, and exchange data without requiring a Cross-Lab cloud account or collapsing security into one privileged service.

The project is Rust-first and built around a shared domain core with native platform integrations where operating-system APIs, permissions, lifecycle, or privileged boundaries require them.

## Why Cross-Lab

Cross-Lab is designed around a few non-negotiable principles:

- **Local-first and peer-to-peer first.** Devices should continue to work together without a mandatory vendor service.
- **Owner-controlled trust.** Device membership, revocation, recovery, and policy remain under the owner's authority.
- **Capability is not permission.** A device advertising support for an operation does not grant another device permission to use it.
- **Least privilege by construction.** UI, networking, plugins, platform adapters, and privileged helpers do not automatically share authority.
- **Transport-independent identity.** IP, Bluetooth, USB, QUIC, relay, or library-specific identities never replace Cross-Lab device identity.
- **Explicit control and data planes.** Bulk or realtime data flows are admitted only after the corresponding control-plane operation is authorized.
- **Native where required.** Desktop uses Rust with GPUI/GPUI Kit; Android uses Kotlin with a shared Rust core; iOS/iPadOS uses Swift with the same shared core boundary.
- **Small, testable components.** Cross-Lab avoids a god daemon and grows crates, services, transports, and platform helpers only when a concrete responsibility requires them.

## Architecture

Cross-Lab uses a Rust workspace and monorepo with clean dependency boundaries between domain logic, platform integration, networking, UI, privileged services, and recovery.

```text
Native Apps / UI
      |
      v
Features and Capability Adapters
      |
      v
Cross-Lab Core
  identity · trust · policy · sessions
      |
      +----------------------+
      v                      v
Protocol / Control Plane   Privileged Boundaries
      |
      v
Transport Adapters
  in-memory simulator · QUIC · future links
```

The initial implementation proves the architecture with a deterministic Core Simulator before introducing real networking or platform-specific feature complexity. Quinn is the first planned real IP transport after the simulator foundation.

For the complete dependency model, security boundaries, platform strategy, protocol rules, and phased roadmap, see the [Master Architecture](./docs/architecture/MASTER-ARCHITECTURE.md).

## Project Status

> **Phase 0 — Architecture and Security Specification is complete.** Cross-Lab is ready to begin Phase 1 implementation with the deterministic Core Simulator and minimal Rust workspace foundation.

Cross-Lab is still pre-release. Production platform capabilities, installers, compatibility guarantees, and end-user releases have not been published yet.

The active implementation state is tracked in [`docs/development/CURRENT.md`](./docs/development/CURRENT.md), with the roadmap in [`docs/development/ROADMAP.md`](./docs/development/ROADMAP.md).

## Documentation

Start with the [documentation index](./docs/README.md).

| Document | Purpose |
|---|---|
| [Master Architecture](./docs/architecture/MASTER-ARCHITECTURE.md) | Authoritative architecture and development baseline |
| [Core Simulator Specification](./docs/architecture/CORE-SIMULATOR.md) | Phase 1 simulator responsibilities and security scenarios |
| [Protocol v1](./docs/protocol/PROTOCOL-V1.md) | Wire compatibility, framing, lifecycle, replay, and canonical transcripts |
| [Threat Model](./security/THREAT-MODEL.md) | Security threats, trust assumptions, and required mitigations |
| [Architecture Decisions](./docs/adr/README.md) | Accepted and future material design decisions |
| [Current Development State](./docs/development/CURRENT.md) | Current milestone, verification state, and exact next task |
| [Development Workflow](./docs/development/WORKFLOW.md) | Repository continuity, branch, verification, and handoff rules |

## Contributing

Cross-Lab is architecture-first and security-sensitive. Read [`CONTRIBUTING.md`](./CONTRIBUTING.md) before changing code or architecture. Research repositories may be used for study and carefully reviewed reuse, but they are not automatic dependencies or source donors.

Security-sensitive reports should follow [`SECURITY.md`](./SECURITY.md) rather than being disclosed in a public issue.

## License

Cross-Lab is dual-licensed under **MIT OR Apache-2.0**, at your option.

See [`LICENSE-MIT`](./LICENSE-MIT) and [`LICENSE-APACHE`](./LICENSE-APACHE).
