# Cross-Lab Plugin Security Boundary

**Status:** Reserved Phase 0 design; runtime deferred  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0

## 1. Purpose

Cross-Lab does not implement a plugin runtime in Phase 0/Phase 1. This document freezes the minimum authority boundary so later extensibility cannot silently inherit core, device-key, or privileged-service authority.

## 2. Core rule

```text
Plugin
  -> explicit Capability API
  -> Policy / Permission Broker
  -> Cross-Lab Core
  -> permitted platform operation
```

A plugin is not an in-process owner/admin principal merely because it is installed.

## 3. Default plugin authority

A future plugin has no default access to:

- owner-root/device-signing/recovery private keys;
- raw device private keys/session secrets;
- unrestricted peer/network sockets;
- unrestricted filesystem;
- other capabilities' private state;
- privileged-service IPC;
- arbitrary shell/command execution;
- recovery namespace;
- update signing/install authority.

Every capability/resource requires an explicit API and owner policy grant.

## 4. Sandboxing direction

WASM/WASI remains the preferred design direction where practical because it offers a narrower default host interface than arbitrary native in-process code. The exact runtime/ABI is deliberately deferred until the capability API is mature.

A future WASM runtime must expose only explicit host functions/capabilities and must use bounded memory, execution, queues, I/O, and cancellation.

## 5. Native extensions

If native extensions are ever supported, they are treated as higher-risk installed software rather than equivalent to sandboxed plugins. Native privileged extensions require a separate installation/authorization model and must not gain privileged helper access through a generic plugin bridge.

## 6. UI extensions

A UI extension may render/request actions but does not receive backend capability authority automatically. UI contribution and capability authorization remain separate.

## 7. Network access

Plugins do not receive raw Cross-Lab session objects or unrestricted network transport handles by default. Network use, if exposed, must be capability-scoped and policy-mediated so plugins cannot bypass peer authentication, trust, control/data-plane authorization, or audit rules.

## 8. Private keys

Plugins never receive exportable owner/device/recovery private key bytes through the plugin API. If a future plugin is allowed to request a signed action, the host signs only a domain-specific, policy-authorized request without exposing the key.

## 9. Privileged operations

Plugins cannot directly connect to the privileged helper as ambient authority. A plugin-requested privileged capability follows the same chain as ordinary requests:

```text
plugin capability request
  -> plugin permission
  -> Cross-Lab policy
  -> typed core operation
  -> authenticated privileged IPC
  -> helper-side validation
```

## 10. Security review trigger

Before any plugin runtime ships, Cross-Lab must define through an ADR/specification:

- plugin package/signing/install model;
- stable capability/host API;
- sandbox runtime;
- filesystem/network/resource capabilities;
- permission UX;
- update/revocation model;
- persistence/state isolation;
- ABI/version compatibility;
- plugin-specific threat model and tests.

Until that work is approved, no production plugin runtime dependency belongs in the foundation workspace.
