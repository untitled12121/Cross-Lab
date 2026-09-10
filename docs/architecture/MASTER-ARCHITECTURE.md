# Cross-Lab Master Architecture & Development Plan

**Document status:** Architecture Baseline — Source of Truth  
**Revision:** 2.1  
**Date:** 2026-09-11  
**Project:** Cross-Lab  
**Scope:** Architecture, security boundaries, repository structure, protocol foundations, platform strategy, development phases, and technology evaluation rules

---

## 1. Document Authority and Change Control

This document defines the authoritative architecture baseline for Cross-Lab. It supersedes the previous master architecture plan and incorporates the approved project naming, desktop stack, repository model, security model, Phase 0 specifications, and Phase 1 foundation decisions.

The purpose of this document is to prevent architectural drift as implementation expands across platforms and capabilities.

The following rules apply:

- Architectural invariants in this document are normative unless explicitly revised.
- Technology choices marked **selected** are approved for the stated scope.
- Technology choices marked **candidate** remain subject to focused prototype and benchmark results.
- Technology choices marked **deferred** must not be introduced simply because they may become useful later.
- New crates, services, daemons, protocol layers, privileged helpers, and dependencies require a concrete responsibility and an identified consumer.
- Any material change to trust, protocol, privilege, recovery, transport, update, or compatibility architecture requires an Architecture Decision Record (ADR) and explicit approval.
- Research repositories are reference material. Their code, protocols, and architecture must not be copied blindly.
- License compatibility, security implications, platform support, maintenance status, and performance impact must be reviewed before code is reused or adapted.
- The smallest architecture that cleanly satisfies the current milestone is preferred over speculative extensibility.

Revision 2.1 incorporates accepted Phase 0 ADRs 0001-0006. Detailed protocol/security mechanics live in their focused specifications; this document records the governing architecture and dependency boundaries.

---

## 2. Vision

**Cross-Lab** is an open-source, local-first, owner-controlled cross-device ecosystem that enables Linux, Windows, macOS, Android, and iOS/iPadOS devices to operate as one coherent system while preserving platform security boundaries and owner authority.

```text
Windows ─ Linux ─ macOS ─ Android ─ iOS/iPadOS
   │        │        │        │          │
   └────────┴────────┴────────┴──────────┘
                    │
                CROSS-LAB
```

### 2.1 Core Philosophy

> **Maximum Owner Control + Least-Privilege Architecture**

Cross-Lab may provide deep control over devices owned by the user, but no component receives more authority than its responsibility requires. Deep integration must be isolated behind narrow, auditable interfaces rather than concentrated in a single privileged process.

---

## 3. Core Principles

Cross-Lab is governed by the following principles:

- Fully open source.
- Transparent and auditable behavior.
- No mandatory Cross-Lab cloud account.
- No mandatory vendor cloud.
- Local-first operation.
- Peer-to-peer first connectivity.
- Optional owner-hosted infrastructure.
- End-to-end encrypted remote communication.
- Owner-controlled device trust.
- Strong device identity.
- Least privilege between internal components.
- Hardware-backed key storage where practical.
- Offline operation where practical.
- Event-driven and lightweight background operation.
- Modular capabilities with explicit versioning.
- Native operating-system integration where required.
- Driver, kernel, root, administrator, SYSTEM, and equivalent authority only where justified.
- Secure, rollback-resistant updates for security-sensitive components.
- Backward-compatible protocol evolution where safe and practical.
- Recovery without dependency on Cross-Lab-operated servers.
- Explicit capability and permission boundaries.
- Sandboxed extension mechanisms where practical.
- Strong developer experience and predictable repository organization.
- Testable architecture with deterministic core behavior.

---

## 4. Architectural Invariants

The following invariants are foundational and must not be weakened for implementation convenience.

### 4.1 Identity Is Independent of Transport

Cross-Lab identity is defined by Cross-Lab credentials and trust relationships. It must not be defined by:

- IP address,
- MAC address,
- Bluetooth address,
- USB device identity,
- QUIC connection identity,
- Iroh endpoint identity,
- libp2p PeerId,
- operating-system account name,
- relay account,
- transport-specific certificate alone.

Transport identities may participate in channel authentication, but they do not replace Cross-Lab owner and device identity.

### 4.2 Capability Does Not Imply Permission

A capability states what a device or platform implementation can perform. Authorization determines whether a specific operation may be performed in a specific context.

```text
Capability = can
Policy     = may
```

A device advertising `screen.capture` does not authorize any peer to capture its screen.

### 4.3 Control Plane and Data Plane Remain Separate

Control-plane operations authorize, create, change, and terminate activity. Data-plane channels carry bulk or realtime payloads.

A data stream must be bound to an authorized control-plane operation or session context.

### 4.4 Privileged Authority Is Isolated

The desktop UI, network-facing agent, plugin runtime, and general capability modules must not automatically inherit root, administrator, SYSTEM, login-provider, driver, or kernel authority.

Privileged operations are exposed through narrow local interfaces and validated again at the privilege boundary.

### 4.5 Recovery Authority Is Separate from Normal Trust

Recovery credentials and recovery operations remain independent from normal device trust. A revoked or lost device may be denied normal ecosystem access while still accepting properly authenticated recovery commands where the platform supports them.

### 4.6 Security Requirements Precede Route Scoring

A route that fails required security or privacy constraints is ineligible. Performance scoring occurs only among eligible routes.

Security is not a soft score that can be traded against lower latency.

### 4.7 Core Domain Semantics Are Not Defined by Third-Party Libraries

Networking, UI, database, FFI, sync, and platform libraries are implementation dependencies. Their identifiers, lifecycle models, and public APIs must not become the Cross-Lab domain model unless explicitly adopted through an ADR.

### 4.8 No God Process

Cross-Lab must not evolve into one large daemon containing networking, privileged operations, UI state, plugins, synchronization, device control, and platform integration in one authority domain.

---

## 5. System Architecture

Cross-Lab is organized as cooperating layers with explicit authority and dependency boundaries.

```text
┌───────────────────────────────────────────────┐
│                Apps / User Interfaces         │
├───────────────────────────────────────────────┤
│              Capability Modules               │
├───────────────────────────────────────────────┤
│             Control Plane / Data Plane        │
├───────────────────────────────────────────────┤
│ Identity / Trust / Policy / Logical Sessions  │
├───────────────────────────────────────────────┤
│          Connection / Transport Layer         │
├───────────────────────────────────────────────┤
│             Native Platform Adapters          │
├───────────────────────────────────────────────┤
│          Privileged Helpers / Drivers         │
├───────────────────────────────────────────────┤
│                  OS / Hardware                │
└───────────────────────────────────────────────┘
```

The initial implementation does not require every layer to be a separate crate or process. Boundaries are introduced when they provide independent security, lifecycle, platform, dependency, or testing value.

---

## 6. Owner, Device, and Trust Model

### 6.1 Owner Identity

The owner identity is the highest normal authority in a Cross-Lab trust domain.

The owner identity must not be represented by one private key reused for every operation. Authority is separated by purpose.

```text
Owner Root Identity
        │
        ├── Device Signing Authority
        ├── Administrative Authority
        └── Recovery Authority
```

The Phase 0 identity/key hierarchy and delegation rules are defined in `docs/architecture/IDENTITY-AND-KEYS.md` and ADR-0002.

### 6.2 Device Identity

Each device has its own device key material and a stable Cross-Lab device identifier bound to authenticated credentials while remaining independent from ordinary key rotation.

A device record may include:

```text
DeviceId
OwnerId
Device public key / credential
Credential epoch
Owner authorization
Device class
Platform
Capability set
Integration level
Trust state
Risk state
Lifecycle state
Recovery state
Metadata
```

Private keys remain local to the device and should use platform hardware-backed storage when practical:

- TPM or platform cryptographic provider on desktop systems,
- Secure Enclave / Keychain-backed protection on Apple platforms,
- Android Keystore / StrongBox where available,
- Windows hardware-backed key providers where available,
- hardware security keys for selected administrative or recovery workflows.

### 6.3 Trust Graph

Cross-Lab uses a trust graph rather than a flat `Owner → Devices` list.

```text
                    Owner Identity
                         │
          ┌──────────────┼───────────────┐
          │              │               │
      Workstation      Laptop           Phone
          │              │               │
       Tablet          Server          Other Device
```

The model must remain extensible to future concepts such as:

- family users,
- work profiles,
- guest access,
- temporary trust,
- shared devices,
- service devices,
- owner-hosted infrastructure.

These concepts are not required for Phase 1 unless needed by the core trust design.

---

## 7. Device State Model

Device state must not be represented by a single enum that mixes unrelated security and lifecycle concepts.

Use orthogonal typed state axes.

Example conceptual model:

```text
ConnectivityState
├── Offline
├── Connecting
└── Online

TrustState
├── Unknown
├── Pending
├── Trusted
└── Revoked

RiskState
├── Normal
├── Limited
├── Suspicious
└── Quarantined

LifecycleState
├── Active
├── Lost
├── Retired
└── Wiping

RecoveryState
├── Normal
├── RecoveryEligible
└── RecoveryOnly
```

The exact states are defined by specifications and tests rather than by UI convenience.

Signed security transitions such as trust establishment, revocation, recovery activation, and credential rotation must be auditable.

---

## 8. Pairing and Trust Establishment

Pairing establishes authenticated trust between a new device and an existing owner trust domain.

A conceptual flow is:

```text
New Device
    │
    ├── discover or receive bootstrap information
    │
    ├── establish provisional authenticated channel
    │
    ├── verify peer and owner intent
    │
    ├── exchange device public credentials
    │
    ├── confirm pairing transcript
    │
    ├── authorize/sign device membership
    │
    ├── exchange capability metadata
    │
    └── establish initial policy
```

Possible bootstrap mechanisms include:

- QR code,
- LAN-assisted pairing,
- BLE-assisted pairing,
- USB-assisted pairing,
- NFC where available,
- existing trusted device,
- hardware security key,
- recovery credential for recovery-specific flows.

Phase 1 pairing profile v1 uses the high-entropy single-use bootstrap model defined by `docs/architecture/PAIRING-TRUST-REVOCATION.md` and ADR-0003. A short numeric pairing code must not be treated as a high-entropy secret; any future numeric-code profile requires an appropriate PAKE or equivalent design with replay and online-guessing protections.

The pairing specification defines:

- transcript contents,
- cryptographic domain separation,
- freshness/nonces,
- mutual confirmation,
- user-visible verification,
- anti-replay rules,
- cancellation behavior,
- failure handling,
- trust record creation,
- audit events.

---

## 9. Capability Model

Each device advertises capabilities it can currently support.

Example:

```text
Android Phone

camera.stream          v2
microphone.stream      v1
notifications.read     v3
files.transfer         v2
biometric.approve      v1
```

```text
Linux Workstation

camera.receive         v2
remote.shell           v3
input.inject           v2
screen.capture         v2
compute.gpu            v1
```

A capability definition includes at minimum:

```text
CapabilityId
Version / compatible version range
Direction where relevant
Required integration level
Runtime availability
Optional limits / feature flags
```

Capability negotiation must account for:

- protocol compatibility,
- platform restrictions,
- runtime permission changes,
- version differences,
- unavailable hardware,
- administrator-managed restrictions.

Unsupported functionality must be reported as unsupported rather than emulated through unsafe or misleading behavior.

---

## 10. Authorization and Policy

Policy evaluation is default-deny.

Policy decisions must separate the primary effect from constraints or obligations.

Conceptual model:

```text
PolicyDecision
├── effect
│   ├── Allow
│   ├── Deny
│   └── Ask
│
└── obligations / constraints
    ├── LocalOnly
    ├── TrustedNetworkOnly
    ├── RemoteAllowed
    ├── SpecificDevice(...)
    ├── SpecificUser(...)
    ├── BiometricApproval
    ├── SecondApproval
    ├── ExpiresAt(...)
    └── RecoveryOnly
```

Authorization context may include:

```text
Authenticated source identity
Destination identity
Capability
Requested operation
Trust state
Device state
Network classification
Physical-presence signal
Requested privilege level
User approval state
Time / expiry context
Security policy
```

Presence, network locality, and device integration level are context signals. They must never silently bypass authentication or explicit high-risk authorization requirements.

---

## 11. Session Architecture

Applications interact with a logical **Cross-Lab Session**, not directly with a Wi-Fi, USB, Bluetooth, Iroh, libp2p, or Quinn connection.

A logical session owns Cross-Lab semantics such as:

- authenticated peer identity,
- negotiated protocol compatibility,
- negotiated capabilities,
- authorization context,
- request/event lifecycle,
- operation identifiers,
- reconnect behavior,
- session shutdown,
- revocation reaction.

A transport connection is replaceable infrastructure beneath the logical session.

Initial versions do not require seamless cross-transport migration. Phase 1 and the first network milestone prove clean disconnect, reconnect, fresh authentication, and state restoration first.

---

## 12. Secure Session Requirements

A Cross-Lab secure session must provide or bind to:

- authenticated peer identity,
- confidentiality on untrusted links,
- integrity protection,
- replay protection,
- fresh session establishment,
- channel binding between transport authentication and Cross-Lab authentication,
- negotiated protocol version,
- explicit close/cancellation semantics,
- revocation handling.

Cross-Lab authorization must not assume that transport encryption alone proves owner trust.

The normative Phase 1 session and transport-neutral contract is defined in `docs/architecture/SESSION-TRANSPORT.md`.

---

## 13. Protocol Architecture

Cross-Lab uses a protocol suite rather than one monolithic protocol.

```text
Cross-Lab Protocol Suite
│
├── Discovery / Bootstrap
├── Pairing
├── Authentication
├── Capability Negotiation
├── Control Request / Response
├── Events
├── Data Stream Negotiation
├── File Transfer
├── Media / Realtime Transport
├── Synchronization
├── Recovery
└── Future Compute / Extension Protocols
```

Not every protocol is implemented in Phase 1.

### 13.1 Protocol Rules

- Wire contracts are versioned.
- Protocol Buffers is the selected v1 ordinary control-plane wire encoding under ADR-0004.
- Security-sensitive signatures/MACs use the independent canonical transcript defined by `docs/protocol/PROTOCOL-V1.md`; protobuf bytes are not the canonical signature representation.
- Unknown optional fields must be handled according to explicit compatibility rules.
- Security-sensitive parsing must be bounded.
- Message lengths and collection sizes must have limits.
- Every request has explicit correlation and cancellation behavior where required.
- Sensitive operations are auditable.
- Parsers for externally supplied data are fuzzed where useful.
- Protocol types do not directly expose transport-library types.

### 13.2 Signed Data

Security operations must not sign arbitrary serializer output without a defined canonical form.

Signed operations require:

- protocol/domain identifier,
- operation type,
- canonical field encoding,
- version,
- signer role,
- freshness or sequence information where required.

Canonical signing transcripts are specified and covered by golden test vectors.

---

## 14. Control Plane and Data Plane

### 14.1 Control Plane

The control plane handles small, security-sensitive operations such as:

- pairing,
- authentication,
- capability discovery,
- permission requests,
- session control,
- start/stop operations,
- lock or administrative requests,
- route changes,
- recovery commands,
- status and audit events.

### 14.2 Data Plane

The data plane carries bulk or realtime data such as:

- files,
- folders,
- video,
- audio,
- screen streams,
- remote desktop payloads,
- backup data,
- large clipboard objects,
- compute results.

### 14.3 Data-Plane Authorization

A data stream must reference an active authorized operation.

Conceptual sequence:

```text
Control request
     │
     ▼
Policy evaluation
     │
     ▼
Authorized operation created
     │
     ▼
OperationId + constraints
     │
     ▼
Data stream requests OperationId
     │
     ├── valid / active / permitted → accept
     └── missing / expired / revoked → reject
```

This rule applies even when control and data travel over the same underlying QUIC connection.

---

## 15. Transport Architecture

Networking is a subsystem independent of capability behavior.

Cross-Lab transport implementations may eventually include:

```text
QUIC
TCP/TLS fallback if justified
LAN
USB
BLE
Wi-Fi Direct / peer-to-peer Wi-Fi
Owner relay
Other platform-specific links
```

The domain model must not depend directly on any one implementation.

### 15.1 Initial Networking Decision

**Selected for the first real IP transport prototype: Quinn / QUIC.**

Reasons:

- narrow transport semantics,
- reliable multiplexed streams,
- optional datagrams,
- mature Rust ecosystem,
- clear separation from Cross-Lab identity and policy,
- straightforward local and chaos testing,
- avoids prematurely adopting a broader P2P architecture.

### 15.2 Iroh

**Status: candidate for later Internet connectivity, NAT traversal, direct-path discovery, and owner-relay evaluation.**

Iroh is not the Cross-Lab identity model and is not selected as the universal Cross-Lab connection fabric at this stage.

Its adoption requires a focused prototype covering:

- direct connection success,
- NAT traversal behavior,
- self-hosted relay support,
- ability to avoid mandatory public infrastructure,
- mobile lifecycle behavior,
- route observability,
- owner control,
- resource usage,
- interoperability with Cross-Lab logical sessions.

### 15.3 rust-libp2p

**Status: candidate/reference for standardized P2P behaviors.**

Relevant areas include:

- relay,
- DCUtR-style hole punching,
- mDNS,
- rendezvous/discovery patterns,
- Multiaddr concepts,
- composable network behavior.

The full libp2p Swarm, DHT, pubsub, and related facilities must not be introduced unless a concrete Cross-Lab requirement justifies them.

### 15.4 Transport Contract

The transport-facing core contract should expose Cross-Lab-neutral concepts such as:

```text
authenticated peer channel
peer/channel binding
open bidirectional stream
open unidirectional stream
datagram support where available
connection metadata
cancel / close
bounded receive/send behavior
```

It must not expose:

```text
Quinn Connection
Iroh Endpoint
libp2p Swarm
Multiaddr
BluetoothPeripheral
USB Device
```

to the Cross-Lab domain layer.

---

## 16. Connection Selection and Adaptive Routing

Adaptive route selection is a later subsystem. It must not complicate Phase 1.

When multiple transports exist, selection occurs in two stages.

### 16.1 Eligibility

A route must first satisfy hard requirements such as:

- required authentication strength,
- required encryption,
- privacy policy,
- owner policy,
- capability requirements,
- platform constraints,
- metered-network restrictions where configured.

### 16.2 Scoring

Eligible routes may then be scored using factors such as:

- latency,
- bandwidth,
- packet loss,
- stability,
- battery cost,
- signal strength,
- metered status,
- user preference.

The route orchestrator must remain separate from capability-specific business logic.

---

## 17. Discovery and Nearby Connectivity

Discovery must be explicit, bounded, and privacy-conscious.

Potential mechanisms include:

- mDNS on local networks,
- BLE advertisements/signalling,
- QR/bootstrap data,
- owner relay/presence service,
- USB enumeration,
- platform-assisted peer Wi-Fi.

Bluetooth should primarily serve:

- discovery,
- proximity signalling,
- wake/bootstrap,
- pairing assistance,
- small control messages where appropriate.

Bulk transfer should prefer higher-bandwidth transports where available.

`btleplug` may be used for supported host-side BLE central/client roles, but Cross-Lab must retain native BLE platform adapters for roles that require peripheral/advertising behavior or APIs not exposed consistently across platforms.

`nusb` is a candidate host-side USB dependency. USB device/gadget functionality on Android, iOS, and other platforms remains platform-specific.

Active network scanning is not the default Cross-Lab discovery strategy.

---

## 18. Platform Process Architecture

Desktop operating systems follow the same authority pattern even when implementation details differ.

```text
Cross-Lab Desktop UI
        │
        ▼
Per-user Cross-Lab Agent
        │
        ├── Cross-Lab Core
        ├── Transport adapters
        ├── Nonprivileged platform adapters
        └── Local state
        │
        ▼
Authenticated local privilege IPC
        │
        ▼
Narrow Privileged Service / Helper
        │
        ▼
OS privileged API / service / driver
```

The privileged service must not become the normal peer-to-peer network endpoint.

---

## 19. Agent Boundary

The **agent** is the normal per-user background runtime.

It may contain:

- Cross-Lab core runtime,
- selected transport adapters,
- nonprivileged platform capability adapters,
- local persistence when introduced,
- UI-facing local IPC,
- privileged-service client,
- audit event production.

It must not acquire privileged authority simply because a capability may occasionally require it.

---

## 20. Privileged Service Boundary

The privileged service exists only where platform functionality requires elevated authority.

It may contain:

- authenticated local IPC,
- strict request decoding,
- caller validation,
- policy/authorization evidence validation where required,
- narrow privileged operation implementations,
- OS-specific privilege checks,
- audit hooks.

It must not contain:

- a general P2P listener,
- peer discovery,
- relay logic,
- plugin runtime,
- arbitrary command execution interfaces,
- desktop UI,
- general sync logic,
- unrelated capability implementations.

The privileged service must expose capability-specific, strongly typed operations rather than generic shell or RPC escape hatches.

---

## 21. Integration Levels

Integration level communicates implementation depth. It is descriptive metadata and must never grant trust or permission by itself.

```text
L1 STANDARD
Normal application permissions

L2 EXTENDED
Background/special user-granted permissions

L3 ADMINISTRATOR
Root/SYSTEM/admin helper

L4 SYSTEM
Login/device-management integration

L5 DRIVER
Virtual hardware or system driver

L6 ADVANCED
Kernel/eBPF/deep platform integration
```

Capabilities advertise the integration level they require and whether that level is currently available.

---

## 22. Desktop Application Architecture

Desktop platforms use **Rust + GPUI + GPUI Kit** across Linux, Windows, and macOS.

Tauri/TypeScript is not the Cross-Lab desktop architecture.

The desktop application follows a feature-first, Next.js-inspired organization:

```text
apps/desktop/src/
├── pages/
│   └── <feature>/
│       ├── page.rs
│       ├── layout.rs
│       └── _components/
├── components/
│   └── ui/
└── features/
```

Responsibilities:

```text
page.rs
  Composes the page from features and components.

layout.rs
  Defines shared page or section layout.

_components/
  Contains components local to one page or feature surface.

components/ui/
  Contains reusable Cross-Lab design-system primitives.

features/
  Contains feature state, controllers, commands, and UI-facing interaction logic.
```

GPUI components must not directly implement:

- network protocols,
- cryptographic operations,
- persistence internals,
- privileged operations,
- large business workflows.

### 22.1 Desktop Design Direction

Cross-Lab desktop UI uses:

- semantic OKLCH design tokens,
- strong light and dark themes,
- radius `0`,
- sharp geometry,
- professional native control-center presentation,
- restrained semantic status colors,
- compact information density,
- clear connection/security/device state,
- keyboard navigation,
- accessibility,
- progressive disclosure for advanced controls.

GPUI Kit is used for compatible primitives and GPUI integration. Cross-Lab owns its design language and must not inherit another application's visual identity wholesale.

---

## 23. Mobile Architecture

### 23.1 Android

Android uses:

```text
Kotlin UI / platform layer
        │
        ▼
Narrow Cross-Lab mobile FFI façade
        │
        ▼
Shared Rust domain/core
```

Platform-specific Android integrations remain in Kotlin or Android-native code where appropriate:

- foreground/background lifecycle,
- notifications,
- MediaProjection,
- Camera,
- microphone,
- Bluetooth,
- Wi-Fi Direct,
- Biometrics,
- Storage Access Framework,
- VPNService where justified.

Support may later distinguish:

- standard Android,
- managed/device-owner mode,
- rooted devices,
- custom ROM/AOSP integration,
- OEM integration.

Advanced modes remain optional.

### 23.2 iOS / iPadOS

iOS and iPadOS use Swift for the native application and platform integrations with a shared Rust core through the same conceptual narrow FFI boundary.

Platform capability reporting must reflect actual Apple platform restrictions.

Potential device classes include:

- normal device,
- managed device,
- supervised device.

Unsupported capabilities are not simulated or advertised as available.

### 23.3 UniFFI

UniFFI is the preferred initial candidate for Kotlin/Swift bindings.

Cross-Lab must expose one deliberately designed mobile façade rather than exporting every internal crate independently. FFI DTOs, callbacks, threading rules, and lifecycle behavior belong at the façade boundary, not in domain crates.

---

## 24. Platform Adapter Model

Platform adapters implement capability-facing operations behind narrow interfaces.

Examples:

```text
clipboard.read
clipboard.write
screen.capture
input.inject
audio.capture
audio.playback
device.lock
notifications.read
camera.capture
```

A platform adapter reports:

- compiled support,
- runtime support,
- permission state,
- integration level,
- relevant limits.

The Cross-Lab core coordinates and authorizes operations but does not contain OS-specific implementation logic.

---

## 25. Platform Strategy

### 25.1 Linux

Linux remains the primary advanced/reference desktop platform.

Likely integrations include:

- DBus,
- PipeWire,
- ALSA where needed,
- uinput/evdev,
- udev,
- PAM,
- Polkit,
- FUSE,
- BlueZ,
- NetworkManager,
- Wayland,
- X11,
- systemd-logind,
- systemd services,
- eBPF only when justified.

### 25.2 Windows

Likely integrations include:

- Win32,
- WinRT,
- WMI where appropriate,
- Media Foundation,
- Windows Audio,
- Windows Hello,
- Credential Provider for later authentication work,
- Bluetooth/WLAN APIs,
- Windows services,
- virtual device/driver APIs only when required.

### 25.3 macOS

The desktop UI remains GPUI. Native platform integrations may use Rust FFI, Objective-C/Swift bridges, XPC, and Apple frameworks where required.

Likely integrations include:

- AVFoundation,
- CoreAudio,
- ScreenCaptureKit,
- LocalAuthentication,
- Network Extension where permitted,
- System Extensions / DriverKit only when necessary.

---

## 26. Capability Families

Capability families organize the ecosystem without forcing all capabilities into one module.

### 26.1 Continuity

- Clipboard
- Files
- Folders
- Sync
- Notifications
- Media handoff
- Messages/calls only where platform APIs permit

### 26.2 Peripheral Sharing

- Camera
- Microphone
- Speakers
- Keyboard
- Mouse
- Touchscreen
- Controller
- Display
- Storage

### 26.3 Remote Control

- Screen view
- Remote desktop
- SSH / terminal
- Approved command execution
- Process/service management
- Power control

### 26.4 Authentication

- Biometric approval
- Phone-assisted computer authentication
- Trusted-device approval
- Hardware security keys
- Login integration

### 26.5 Networking

- Internet sharing
- Private device network
- Service exposure
- Port/service forwarding
- Owner relay
- Owner hub

### 26.6 Compute

- CPU jobs
- GPU jobs
- AI inference
- Compilation
- Rendering
- Transcoding
- Compression
- Backup processing

These families describe future scope. They do not imply that all modules are created in the initial workspace.

---

## 27. Synchronization Architecture

Synchronization is not part of the initial core simulator except where protocol state must be exchanged.

### 27.1 File Synchronization

Syncthing is reference material for:

- block-oriented transfer,
- resumability,
- version vectors,
- conflict handling,
- disconnect/reconnect behavior,
- backpressure,
- file-system edge cases.

Cross-Lab must not adopt the Syncthing daemon as its universal architecture.

### 27.2 Structured Local-First Data

Automerge is a candidate for genuinely mergeable structured data where CRDT semantics are appropriate.

Automerge or any CRDT must not resolve authoritative security state such as:

- owner identity,
- trust establishment,
- revocation,
- recovery authority,
- security policy,
- signed administrative actions.

Those require explicit authenticated state transitions.

---

## 28. Realtime Media and Remote Control

Media and remote control are later phases and remain outside the initial core.

Research guidance:

- CPAL is a candidate low-level audio I/O backend.
- XCap is a candidate desktop screen-capture backend behind platform capability interfaces.
- Enigo is a candidate input-injection backend where platform policy permits.
- scrcpy is reference/wrapping material for Android screen/control architecture; its internal protocol is not a Cross-Lab public protocol.
- RustDesk is research material for media, relay, remote-control lifecycle, and cross-platform implementation patterns.
- FreeRDP is a future interoperability candidate for RDP rather than reimplementing RDP.

Realtime features must retain the same identity, policy, operation authorization, audit, and privilege boundaries as other capabilities.

---

## 29. Cross-Device Authentication

Phone-assisted computer authentication is a later capability family.

PC Bio Unlock is reference material for:

- challenge/response flows,
- replay resistance,
- local network operation,
- privileged login integration,
- retaining conventional login fallback.

Cross-Lab must prefer native OS authentication mechanisms where available and must not design a general architecture that requires storing reusable user login passwords in the Cross-Lab agent.

---

## 30. Presence Engine

Presence may use signals such as:

- BLE proximity,
- local network visibility,
- USB attachment,
- device activity,
- owner confirmation.

Presence can influence policy but must not independently satisfy critical authentication.

Examples:

```text
Phone leaves     → lock computer according to configured policy
Phone returns    → request authentication or approval
Unknown network  → restrict selected capabilities
```

Presence decisions must be debounced, auditable where security relevant, and resilient to spoofed or noisy signals.

---

## 31. Owner-Hosted Infrastructure

Cross-Lab has no mandatory cloud dependency.

Optional owner-hosted infrastructure may be introduced later.

### 31.1 Cross-Lab Relay

A minimal relay may provide:

- presence/signalling,
- NAT assistance,
- encrypted packet relay,
- wake messages.

The relay must not require access to:

- owner root keys,
- device private keys,
- plaintext files,
- plaintext media streams.

### 31.2 Cross-Lab Hub

A larger optional owner-controlled hub may later provide:

- relay,
- local discovery assistance,
- encrypted sync cache,
- encrypted backup,
- notification queue,
- device status,
- update mirror,
- automation,
- compute coordination.

Supported deployment targets may include NAS systems, home servers, mini PCs, VPS instances, and similar owner-controlled environments.

Multiple hubs should remain possible.

---

## 32. Recovery Plane

Recovery is specified early even though full implementation occurs later.

```text
Normal Trust Plane
       X
       │ revoked / unavailable
       │
Recovery Authority
       │
       ▼
Recovery-capable Device
```

Potential recovery operations include:

- locate where the platform permits,
- lock,
- sound,
- display recovery message,
- request status,
- revoke credentials,
- rotate credentials,
- recover approved encrypted data,
- wipe where the platform permits.

Recovery commands must be:

- owner-authorized,
- cryptographically separated from normal device commands,
- narrowly scoped,
- replay-resistant,
- visible/auditable,
- platform-compliant,
- unusable as covert surveillance functionality.

The normative recovery authority and command model is defined in `docs/architecture/RECOVERY-UPDATE-SECURITY.md`.

---

## 33. Secure Updates

Update security is part of the architecture from the beginning because Cross-Lab will eventually contain privileged components.

The update model must support:

- signed metadata and artifacts,
- multiple signing roles,
- version verification,
- rollback protection,
- freeze protection,
- key-compromise recovery,
- update channels,
- emergency security updates,
- platform driver/signing requirements,
- protocol compatibility awareness.

A TUF-style architecture with separated root/targets/snapshot/timestamp roles is selected under ADR-0005. Production root trust uses the threshold policy defined there.

`tough` remains a candidate implementation dependency when the updater is implemented. It is not required for Phase 1.

---

## 34. Plugin and Extension Architecture

Plugins are deferred until the capability API and security boundaries are stable.

A future plugin model follows:

```text
Plugin
  │
  ▼
Capability API
  │
  ▼
Permission / Policy Broker
  │
  ▼
Cross-Lab Core
```

Plugins must not inherit:

- owner root authority,
- device private keys,
- arbitrary network access by default,
- privileged-service authority,
- unrestricted filesystem access,
- unrestricted access to other capabilities.

WASM/WASI is the preferred design direction where practical. Fungi and GPUI Kit extension mechanisms are research references, not automatically the Cross-Lab plugin ABI.

Native privileged extensions, if ever supported, require explicit installation and a separate owner authorization model.

---

## 35. Observability and Audit

The owner-facing system should eventually expose:

- devices online/offline,
- trust and risk state,
- active transport,
- active sessions,
- capabilities in use,
- throughput,
- relevant CPU/RAM/battery data,
- permission decisions,
- remote requests,
- authentication attempts,
- security events,
- recovery operations,
- update state.

Sensitive actions produce structured audit events containing only the information necessary for accountability.

Typical fields include:

```text
Source DeviceId
Destination DeviceId
User / authority where applicable
CapabilityId
OperationId
Timestamp
Authorization result
Transport class
Result
```

Audit logs must never contain:

- private keys,
- reusable credentials,
- raw authentication secrets,
- sensitive payloads by default,
- plaintext file/media contents unless a capability explicitly requires user-requested recording.

The detailed audit, privacy, and redaction contract is defined in `docs/architecture/AUDIT-PRIVACY.md`.

---

## 36. Repository Architecture

Cross-Lab uses a monorepo and Rust workspace.

The repository grows according to actual boundaries rather than pre-creating every future crate.

### 36.1 Initial Phase 1 Workspace

```text
crosslab/
├── Cargo.toml
├── rust-toolchain.toml
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── SECURITY.md
├── CONTRIBUTING.md
│
├── crates/
│   ├── crypto/
│   ├── identity/
│   ├── policy/
│   ├── protocol/
│   └── core/
│
├── apps/
│   └── sim/
│
├── docs/
│   ├── architecture/
│   ├── protocol/
│   └── adr/
│
└── security/
    └── THREAT-MODEL.md
```

The first real networking milestone adds only the required transport implementation, initially:

```text
transports/
└── quic/
```

### 36.2 Expected Long-Term Shape

The repository may later grow toward:

```text
crosslab/
├── apps/
│   ├── desktop/
│   ├── android/
│   └── ios/
│
├── crates/
│   ├── crypto/
│   ├── core/
│   ├── identity/
│   ├── policy/
│   ├── protocol/
│   ├── recovery/          # when independently justified
│   ├── sync/              # when independently justified
│   └── update/            # when independently justified
│
├── transports/
├── platforms/
├── services/
├── modules/
├── sdk/
├── plugins/
├── design/
├── tests/
├── security/
├── docs/
└── tools/
```

Empty directories and speculative crates are not required merely to match this future diagram.

---

## 37. Initial Crate Responsibilities

### 37.1 `crosslab-crypto`

Owns only narrow implementation-neutral cryptographic mechanics shared by multiple domain crates:

- secure random helpers/interfaces;
- cryptographic profile/algorithm identifiers;
- Ed25519 v1 sign/verify wrappers;
- BLAKE3 domain-separated digest/fingerprint helpers;
- canonical transcript v1 builder/encoding/digest primitives;
- constant-time verification helpers where required;
- sensitive wrappers needed to prevent accidental secret logging/exposure.

It does not own owner/device credentials, trust records, policy, protocol messages, sessions, sockets, persistence, platform key stores, or recovery/update business semantics. This boundary is governed by ADR-0006.

### 37.2 `crosslab-identity`

Owns security-domain identity semantics such as:

- `OwnerId`,
- `DeviceId`,
- key and credential domain types,
- owner-authorized device credentials,
- key roles,
- credential epochs,
- fingerprints/identifiers as domain values,
- revocation identifiers,
- identity-specific signed-object construction/verification.

Private-key representations remain encapsulated. Generic cryptographic mechanics live in `crosslab-crypto`; identity business semantics remain here.

### 37.3 `crosslab-policy`

Initially owns:

- trust records,
- revocation state,
- capability identifiers/domain policy types where appropriate,
- permission grants,
- authorization context,
- policy evaluation,
- policy effects,
- obligations and constraints.

If trust and capability domains grow independently, they may later be extracted into dedicated crates through an ADR.

### 37.4 `crosslab-protocol`

Owns wire-contract concerns only:

- protocol versions,
- envelopes,
- request/response/event messages,
- pairing/authentication protocol messages,
- capability negotiation messages,
- stream-open negotiation,
- protocol error codes,
- compatibility rules,
- object-specific canonical transcript mappings where protocol/session-owned,
- wire/domain conversion.

It does not own sockets, platform APIs, UI, policy evaluation, or privileged actions.

### 37.5 `crosslab-core`

Owns the unprivileged coordination engine:

- node state machine,
- session lifecycle,
- capability registry,
- authentication orchestration,
- control request dispatch,
- event dispatch,
- operation authorization lifecycle,
- data-stream authorization,
- disconnect/reconnect handling,
- revocation reaction,
- bounded queues,
- shutdown and cancellation.

It must remain platform-independent and must not implement privileged OS operations.

### 37.6 Transport Abstraction

The transport contract begins as a narrow API seam used by the core rather than an empty crate created for theoretical purity.

A dedicated transport abstraction crate is introduced when multiple concrete transports or dependency isolation make that boundary useful.

---

## 38. Dependency Direction

Dependency flow must point inward toward stable domain semantics.

Conceptually:

```text
crosslab-crypto
      │
      ▼
crosslab-identity
      │
      ▼
crosslab-policy
      │
      ├──────────┐
      ▼          ▼
crosslab-protocol│
      │          │
      └────┬─────┘
           ▼
      crosslab-core
           │
     ┌─────┴────────┐
     ▼              ▼
simulator      concrete adapters
```

`crosslab-protocol` may depend directly on `crosslab-crypto` for shared canonical/digest helpers and on domain crates for validated domain values. `crosslab-core` may depend on `crosslab-crypto` only for session/security orchestration mechanics not owned by another domain.

Forbidden examples include:

```text
crypto   -> policy business rules
identity -> GPUI
identity -> Quinn
policy   -> platform APIs
protocol -> Bluetooth APIs
core     -> Android/Kotlin
core     -> GPUI
core     -> privileged service implementation
core     -> SQLite implementation
```

Application and adapter crates may depend on domain crates. Domain crates do not depend on applications.

Circular crate dependencies are not permitted.

---

## 39. Dependency Policy

Dependencies are added only when required by a current milestone.

Rules:

- Use latest stable, maintained, compatible versions at implementation time.
- Verify current versions rather than relying on remembered version numbers.
- Pin and manage shared dependency versions at workspace level where appropriate.
- Avoid abandoned dependencies.
- Prefer small, focused dependencies over broad frameworks when the broader framework is unnecessary.
- Review transitive dependency impact for security-sensitive components.
- Prefer audited or widely used cryptographic libraries; do not implement custom cryptographic primitives.
- Keep feature flags minimal.
- Avoid duplicate runtimes and duplicate TLS/crypto stacks unless justified.
- Generate SBOMs as release engineering matures.

### 39.1 Initial Dependency Budget

The Phase 1 simulator should begin with only dependencies required for:

- async execution,
- cryptographic signatures/key generation,
- hashing/fingerprints where specified,
- protocol serialization,
- typed errors,
- cancellation if required,
- application tracing.

The following are intentionally **not** initial dependencies unless a current task requires them:

```text
SQLite
Iroh
rust-libp2p
GPUI
UniFFI
Automerge
btleplug
nusb
WASM runtime
CPAL
XCap
Enigo
tough
```

Quinn is introduced at the first real network milestone.

Exact versions for Phase 1 dependencies are verified immediately before implementation rather than frozen in this architecture document.

---

## 40. Research Repository Reuse Policy

Uploaded research repositories are used according to five modes:

```text
REUSE
  Depend on the maintained upstream library where suitable.

WRAP
  Isolate the dependency behind a Cross-Lab adapter or façade.

STUDY
  Learn protocol, algorithm, architecture, lifecycle, or testing patterns.

REIMPLEMENT
  Implement Cross-Lab-owned semantics informed by research without inheriting unsuitable APIs.

AVOID
  Do not adopt the architecture or code for the stated role.
```

Before any code is adapted rather than merely studied, verify:

- repository license,
- dependency licenses,
- compatibility with Cross-Lab's `MIT OR Apache-2.0` license,
- upstream maintenance,
- platform support,
- security history,
- API stability,
- performance characteristics.

Cross-Lab's project license does not make incompatible third-party source reusable. Copyleft, noncommercial, or otherwise restrictive research source remains study-only unless a specific compatibility/legal review permits adaptation.

---

## 41. Research Reuse Matrix

| Project | Cross-Lab area | Direction | Architectural use |
|---|---|---|---|
| GPUI Kit | Desktop UI | Reuse / wrap | GPUI integration and reusable UI primitives; Cross-Lab owns visual language |
| Quinn | IP transport | Reuse | First QUIC transport prototype |
| Iroh | Internet/NAT/relay | Study / prototype later | NAT traversal, relay, path discovery, remote connectivity |
| rust-libp2p | P2P networking | Study / selective reuse | Relay, hole punching, discovery, transport concepts |
| Fungi | Networking/agent/extensions | Study / adapt patterns | Stream authorization, negotiation, testing, extension isolation |
| KDE Connect | Cross-device ecosystem | Study / reimplement concepts | Capability/plugin decomposition, multiplexing, interoperability lessons |
| COSMIC Connect Core | Shared Rust ecosystem | Study | Rust + UniFFI cross-device implementation lessons; avoid overly broad core boundary |
| UniFFI | Mobile FFI | Reuse later | Narrow Kotlin/Swift façade over Rust core |
| nusb | USB | Wrap later | Host-side USB transport/platform implementation |
| btleplug | BLE | Wrap partially | BLE central/client roles; native adapters required for missing peripheral/platform roles |
| Flying Carpet | Nearby/offline | Study / reimplement patterns | BLE bootstrap, hotspot/peer-Wi-Fi roles, offline transfer design |
| Syncthing | File sync | Study | Resume, block sync, versioning, reconnect, conflict behavior |
| Automerge | Structured sync | Reuse narrowly later | CRDT state where merge semantics are valid; never authoritative security state |
| scrcpy | Android control | Wrap/study later | Android media/control process split; internal protocol remains isolated |
| Another | Android control | Study | Adapter/version-seam lessons around scrcpy integration |
| PC Bio Unlock | Authentication | Study / reimplement security patterns | Challenge-response and login integration boundaries |
| RustDesk | Remote access | Study | Relay, media, remote-control lifecycle, cross-platform architecture |
| FreeRDP | RDP interoperability | Wrap later | Existing RDP implementation rather than protocol reimplementation |
| CPAL | Audio | Reuse later | Cross-platform low-level audio I/O |
| XCap | Screen capture | Wrap later | Desktop capture backend behind capability boundary |
| Enigo | Input | Wrap later | Input injection behind platform/permission boundary |
| osquery | Monitoring | Study / optional integration | Typed system information and extension patterns; avoid baseline daemon dependency |
| Reconya | Discovery/monitoring | Study only | Active scanning lessons; not default Cross-Lab discovery architecture |
| tough | Secure update | Reuse later | TUF-style update implementation candidate |

License details are verified from the exact upstream revision before reuse. Research classification does not itself grant permission to copy source code.

---

## 42. Security Architecture

Security is part of feature design, not a final hardening phase.

Minimum security rules:

- Authenticate identity before sensitive actions.
- Evaluate trust before granting ecosystem access.
- Evaluate capability, permission, and contextual policy before every protected operation.
- Revalidate sensitive requests at privilege boundaries.
- Never log private keys, credentials, authentication secrets, or sensitive payloads.
- Minimize and isolate `unsafe` Rust.
- Treat externally supplied lengths/counts as untrusted.
- Bound queues, streams, and message sizes.
- Use explicit cancellation and shutdown.
- Prevent stale authorization from surviving revocation.
- Separate recovery credentials from ordinary trust credentials.
- Use domain-separated signing and authenticated transcripts.
- Avoid ambient privileged authority.
- Prefer deny-by-default behavior when state is incomplete or ambiguous.

---

## 43. Testing Architecture

Testing begins with the core and follows every boundary.

Cross-Lab requires:

- unit tests,
- integration tests,
- protocol golden tests,
- cross-version compatibility tests,
- parser fuzzing where useful,
- cryptographic test vectors,
- privilege-boundary tests,
- network chaos tests,
- disconnect/reconnect tests,
- NAT/relay tests when remote networking is introduced,
- low-bandwidth tests,
- packet-loss tests,
- revocation tests,
- recovery tests,
- cross-platform interoperability tests.

Security-sensitive bugs receive regression tests.

Tests should prefer observable behavior over implementation details.

---

## 44. Phase 0 — Architecture and Security Specification

Phase 0 freezes foundational contracts before production capability work begins.

Required specifications:

1. Architecture and dependency-direction rules.
2. Threat model and trust boundaries.
3. Owner/device identity hierarchy.
4. Credential formats, key roles, rotation, and epochs.
5. Pairing transcript and bootstrap authentication.
6. Trust establishment and revocation.
7. Capability identifiers and compatibility rules.
8. Authorization effect/obligation model.
9. Protocol envelope and version negotiation.
10. Error, timeout, retry, cancellation, and replay semantics.
11. Secure-session and channel-binding contract.
12. Control-plane/data-plane authorization relationship.
13. Transport interface and security requirements.
14. Recovery authority and recovery protocol namespace.
15. Update trust/TUF role model and rollback policy.
16. Canonical signing transcript rules and golden vectors.
17. Audit/redaction/privacy rules.
18. Plugin security boundary as a reserved future design.
19. Repository license and third-party reuse policy.

Phase 0 does not implement desktop/mobile features. The completed coverage and accepted ADR register are recorded in `docs/plans/phase-0/PHASE-0-CLOSEOUT.md`.

---

## 45. Phase 1 — Core Simulator

The Core Simulator proves the architecture without OS-specific complexity.

Two simulated devices must be able to:

```text
create identities
pair
authenticate
establish trust
exchange capabilities
evaluate permissions
establish a secure logical session
send control requests/responses
send events
authorize data operations
open data streams
disconnect
reconnect and authenticate again
revoke trust
terminate or reject invalid sessions
```

### 45.1 Required Negative Scenarios

The simulator and protocol tests must cover at minimum:

```text
tampered credential
unknown owner
invalid owner authorization
expired or invalid credential epoch
replayed authentication data
wrong peer/channel binding
unsupported capability
unauthorized capability
policy denial
stream opened without OperationId
expired OperationId
revoked OperationId
malformed frame
oversized frame / collection
revocation during active session
reconnect after revocation
queue saturation
cancellation during operation or stream setup
clean shutdown with active tasks
```

### 45.2 In-Memory First

The first simulator transport is deterministic and in-memory. It validates Cross-Lab state machines, authorization, protocol semantics, backpressure, and lifecycle behavior without conflating them with network implementation.

The simulator must not implement toy cryptography and call it secure. Production cryptographic primitives are used where security semantics are being tested.

The detailed workspace, scenario, and test obligations are defined in `docs/architecture/CORE-SIMULATOR.md`.

---

## 46. First Network Milestone — Quinn

After the in-memory simulator is stable, add a Quinn-based QUIC transport.

The milestone must prove:

- encrypted loopback/local communication,
- Cross-Lab peer authentication over the transport,
- channel binding,
- multiplexed control and data streams,
- bounded stream handling,
- clean disconnect,
- reconnect and reauthentication,
- cancellation,
- revocation behavior,
- network fault tests.

Quinn connection objects remain inside the transport implementation.

---

## 47. Networking Evaluation Checkpoint

After the Quinn baseline is measured, perform an Internet-connectivity prototype rather than choosing a broad P2P framework by assumption.

Evaluate Iroh first for:

- NAT traversal,
- relay behavior,
- self-hosted operation,
- no mandatory Cross-Lab public service,
- mobile lifecycle,
- route observability,
- connection recovery,
- resource usage,
- owner control.

Evaluate rust-libp2p where standardized relay/discovery/hole-punching behavior could materially improve the architecture.

Record the selected remote-connectivity approach in an ADR.

Cross-Lab logical sessions remain independent from whichever implementation wins.

---

## 48. Development Roadmap

### Phase 0 — Specification

Complete. The frozen baseline is recorded by the Phase 0 specifications, accepted ADRs, and closeout review.

### Phase 1 — Core Simulator

Prove identity, trust, authorization, protocol, sessions, streams, reconnect, and revocation with simulated peers.

### Phase 2 — Linux + Android MVP

Target:

- discovery,
- secure pairing,
- automatic connection,
- device status,
- permissions,
- clipboard,
- file transfer with resume,
- notifications,
- audit,
- revocation.

A future Cross-Lab 0.1 milestone succeeds when two real devices can perform these operations without mandatory cloud infrastructure.

### Phase 3 — Adaptive Networking

Add only after the base transport/session model is proven:

- Ethernet/LAN optimization,
- BLE discovery,
- Wi-Fi Direct/platform peer Wi-Fi,
- USB,
- Internet P2P,
- NAT traversal,
- owner relay,
- route scoring,
- controlled route switching.

### Phase 4 — Windows

Add Windows agent, service, GPUI desktop integration, and capability adapters.

### Phase 5 — Realtime Media and Peripherals

Add:

- camera,
- microphone,
- audio,
- touchpad,
- keyboard,
- controller,
- virtual camera/microphone where justified.

### Phase 6 — Display and Remote Control

Add:

- screen streaming,
- remote desktop,
- terminal/SSH integrations,
- approved remote commands,
- process/service control,
- display extension where feasible.

### Phase 7 — Owner-Hosted Infrastructure

Release owner-controlled relay and hub components.

### Phase 8 — Cross-Device Authentication

Add:

- Linux PAM integration,
- Windows Credential Provider integration,
- phone-assisted approval,
- biometric approval,
- hardware security key integration.

### Phase 9 — Recovery

Implement the previously specified recovery plane:

- lost mode,
- quarantine,
- recovery status,
- revocation,
- credential rotation,
- lock,
- locate where supported,
- wipe where supported.

### Phase 10 — Apple Mobile Completion

Complete feasible iOS/iPadOS capability adapters and platform-specific restrictions.

The macOS desktop application already follows the shared GPUI desktop strategy and is not deferred to this phase as a UI architecture.

### Phase 11 — Advanced Platform Integration

Add only when required:

- virtual devices,
- System Extensions,
- drivers,
- eBPF,
- kernel modules,
- managed-device integrations,
- AOSP/OEM integrations.

### Phase 12 — Distributed Compute

Add:

- remote job execution,
- CPU/GPU workloads,
- AI inference,
- build workloads,
- rendering,
- transcoding.

### Phase 13 — SDK and Ecosystem

Add stable SDKs and plugin APIs only after protocol/capability contracts are mature.

---

## 49. Implementation Milestones for the Foundation

The initial implementation sequence is deliberately small and verifiable.

### M0 — Phase 0 Specifications

**Complete.** Deliver the architecture, threat model, protocol/security specs, ADR structure, license decision, and dependency policy.

### M1 — Repository Foundation

Create the minimal Rust workspace, formatting/lint/test configuration, and simulator application shell.

### M2 — Identity

Implement typed identities, key generation, device credentials, verification, and negative cryptographic tests.

### M3 — Trust and Policy

Implement trust records, revocation, capability identifiers required by policy, and pure default-deny policy evaluation.

### M4 — Protocol

Implement versioned envelopes, pairing/authentication messages, capability negotiation, control/events, stream negotiation, test vectors, and parser hardening.

### M5 — Deterministic Two-Peer Simulator

Prove pairing, authentication, capability exchange, policy, control requests, and events using the in-memory transport.

### M6 — Authorized Data Streams

Add operation-bound streams, bounded queues, backpressure, cancellation, and shutdown behavior.

### M7 — Failure and Security Lifecycle

Prove reconnect, replay rejection, active revocation, reconnect-after-revocation denial, malformed input behavior, and resource limits.

### M8 — Quinn Transport

Add real QUIC communication and network integration tests.

### M9 — Remote Networking ADR

Prototype and measure Iroh and, where justified, rust-libp2p approaches against the Quinn baseline. Select the remote connectivity architecture through an ADR.

### M10 — First Platform Vertical Slice

Begin Linux desktop and Android integration only after the core/session/protocol foundation is stable.

---

## 50. Development and DX Rules

### 50.1 Repository Organization

- Prefer feature-first organization and vertical slices.
- Keep related state, interaction logic, errors, views, and tests close together.
- Do not create global `models/`, `services/`, or `controllers/` dumping grounds.
- Keep files and modules focused.
- Extract boundaries when they represent real ownership or dependency differences.

### 50.2 Code Quality

- Use idiomatic stable Rust.
- Prefer strong types over stringly typed states and identifiers.
- Prefer explicit composition over clever abstraction.
- Avoid unnecessary traits.
- Avoid excessive cloning and allocation.
- Avoid global mutable state.
- Use bounded async queues.
- Use explicit cancellation/shutdown.
- Prefer streaming and backpressure for large data.
- Avoid unnecessary polling.
- Remove dead and duplicated code.
- Refactor touched code when it directly improves the active feature.

### 50.3 Comments and Documentation

Comments should be short and explain **why** a non-obvious decision exists.

Do not add comments that merely restate code.

Public APIs and unusual security/platform decisions should be documented where useful.

### 50.4 Verification

Before a milestone is considered complete, run the relevant equivalent of:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
security/integration/fuzz checks required by the milestone
```

Exact commands may be adjusted if feature combinations or platform constraints make `--all-features` inappropriate; such exceptions must be documented.

### 50.5 Automation

Do not add an `xtask` crate merely as convention.

Introduce `cargo xtask` or equivalent when the project has real cross-platform automation requirements such as:

- code/schema generation,
- mobile binding generation,
- packaging,
- release signing,
- test-lab orchestration,
- reproducible build workflows.

---

## 51. Open Architecture Decisions

Phase 0 resolved the repository license, v1 identity cryptographic profile, v1 pairing bootstrap, v1 wire/canonical-signing model, v1 update trust model, and the focused Phase 1 `crosslab-crypto` boundary through ADR-0001 through ADR-0006.

The following items remain intentionally unresolved until evidence is available.

| Decision | Current status | Resolution point |
|---|---|---|
| Remote NAT/relay implementation | Quinn baseline + Iroh candidate + libp2p alternative | M9 networking ADR |
| Persistent metadata database | Deferred | First feature requiring durable structured state |
| Dedicated transport-abstraction crate | Deferred | When multiple transports justify extraction |
| Plugin runtime | Deferred | After capability ABI/security model stabilizes |
| CRDT use | Deferred | First structured state with genuine merge semantics |
| TCP/TLS fallback | Deferred | Only if compatibility or network evidence justifies it |
| WireGuard integration | Deferred | Only if private-network capability requires it |
| Driver/kernel components | Deferred | Only where safer APIs cannot satisfy a concrete capability |

Unresolved decisions must not be silently treated as approved implementation choices.

---

## 52. Architecture Decision Record Policy

An ADR is required when a decision materially affects any of the following:

- public protocol compatibility,
- identity or trust semantics,
- cryptographic formats,
- privilege boundaries,
- recovery authority,
- update trust,
- persistent data format,
- transport/session abstraction,
- mandatory infrastructure,
- cross-platform API contracts,
- plugin authority,
- major repository restructuring.

Each ADR should state:

```text
Context
Decision
Alternatives considered
Security impact
Compatibility impact
Operational impact
Consequences
Status
```

Superseded ADRs remain in the repository for historical context.

---

## 53. Governance and Security Documentation

The repository should maintain:

```text
README.md
MASTER-ARCHITECTURE.md
SECURITY.md
THREAT-MODEL.md
protocol specifications
CONTRIBUTING.md
CODEOWNERS
ADRs
Release signing policy
Security disclosure process
Dependency policy
SBOM/reproducible-build documentation as the project matures
```

Security-sensitive code receives stricter review than ordinary UI code.

---

## 54. Foundation Success Criteria

The architecture foundation is considered proven when:

1. Two independent simulated devices create valid identities.
2. Pairing produces authenticated owner-approved trust.
3. Peers mutually authenticate on a fresh session.
4. Capability negotiation handles compatible and incompatible versions correctly.
5. Policy is default-deny and produces structured authorization decisions.
6. Control messages and events are correlated, bounded, and cancelable where required.
7. Data streams cannot open without a valid authorized operation.
8. Disconnect/reconnect does not bypass fresh authentication requirements.
9. Revocation terminates or invalidates affected active authority and prevents future trusted sessions.
10. Malformed/replayed/oversized input is rejected without resource exhaustion.
11. The same domain behavior passes over the deterministic in-memory transport and the Quinn transport.
12. No UI, platform API, database, P2P framework, or privileged service is required to validate core semantics.

---

## 55. Final System Model

```text
                         CROSS-LAB
                             │
                       Owner Identity
                             │
                         Trust Graph
                             │
                        Policy Engine
                             │
                     Logical Sessions
                             │
            ┌────────────────┴────────────────┐
            │                                 │
      CONTROL PLANE                      DATA PLANE
            │                                 │
   auth / commands / events          files / media / streams
            │                                 │
            └────────────────┬────────────────┘
                             │
                 Connection / Transport Layer
                             │
       QUIC / LAN / USB / BLE / Internet / Relay
                             │
            Linux / Windows / macOS / Android / iOS
                             │
               Capability / Platform Adapters
                             │
               Narrow Privileged Boundaries
                             │
                        OS + Hardware
```

---

## 56. Cross-Lab Architectural Rule

> **Identity determines who a participant is.**  
> **Trust determines whether that participant belongs to the relevant trust domain.**  
> **Capabilities describe what a device can perform.**  
> **Policy determines what an authenticated participant may perform in the current context.**  
> **Logical sessions coordinate secure Cross-Lab interaction independently of the current transport.**  
> **The control plane authorizes operations.**  
> **The data plane carries authorized payloads.**  
> **The connection layer determines how eligible traffic travels.**  
> **Platform adapters perform operating-system-specific work.**  
> **Privileged helpers receive only the additional authority required for narrow operations.**  
> **Recovery authority remains available independently of normal trust where the platform permits it.**

This rule is the architectural foundation of Cross-Lab. Feature work must conform to it rather than redefining the foundation around individual platform APIs, libraries, or short-term implementation convenience.
