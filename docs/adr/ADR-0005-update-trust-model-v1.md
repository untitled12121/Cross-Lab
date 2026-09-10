# ADR-0005: Update trust model v1

**Status:** Accepted  
**Date:** 2026-09-11  
**Accepted:** 2026-09-11

## Context

Cross-Lab will eventually install security-sensitive and privileged components across multiple operating systems. A compromised download server, mirror, CDN, or owner hub must not be able to authorize arbitrary binaries or silently roll clients back to known-vulnerable releases. The Master Architecture selects a TUF-style design direction and names `tough` only as a candidate implementation.

## Decision

Use a TUF-style update trust model with distinct roles:

```text
Root
Targets
Snapshot
Timestamp
Delegated targets / release channels
```

Production policy requirements:

- Root is the offline trust anchor and uses a threshold of at least **2-of-3** independently protected root keys before the first production release.
- Targets metadata authorizes release artifacts and their hashes/lengths.
- Snapshot binds consistent versions of targets/delegated metadata.
- Timestamp provides freshness and freeze protection.
- Stable/beta/nightly or platform/component channels use delegated target roles rather than one universal online key where practical.
- Update distribution infrastructure holds no root private key and is not trusted merely because TLS succeeds.
- Clients persist the highest trusted metadata versions and installed component version state required to reject rollback.
- Normal production updates are monotonic/fix-forward. If functionality must revert, publish a new higher application version containing the desired code rather than authorizing a version-number downgrade.
- Expired/frozen metadata fails closed for unattended privileged installation; recovery requires an explicit trusted metadata/root recovery procedure.
- Root rotation follows TUF-style old-root/new-root trust transition rules and is not performed through ordinary application update metadata alone.

Platform vendor signing/notarization requirements remain additional checks and do not replace Cross-Lab update metadata verification.

## Alternatives considered

### Single release-signing key

Simpler but creates one compromise point and weak recovery/role separation.

### HTTPS/TLS-only updates

Rejected. Distribution-path authentication does not defend against repository compromise, rollback/freeze, or signing-key separation requirements.

### Custom update metadata protocol

Rejected. TUF exists specifically to address update-system compromise classes and should be reused conceptually/through a maintained implementation where suitable.

### Lower root threshold for convenience

Acceptable only for pre-release development artifacts that are not presented as production-secure updates. Production root threshold remains at least 2-of-3.

## Security impact

Role separation, offline threshold root keys, signed target metadata, snapshot consistency, timestamp freshness, and monotonic client state reduce repository compromise, rollback, freeze, and single-key compromise risk.

Compromise of sufficient root threshold keys remains catastrophic and requires a separately documented incident/recovery process.

## Compatibility impact

Update metadata format and trusted root become long-lived compatibility artifacts. Clients must preserve root/version transition compatibility across supported release windows.

## Operational impact

Before production release the project must provision independent root keys, define custody/backup procedures, configure online targets/snapshot/timestamp roles, and integrate OS-specific signing/notarization/driver requirements.

`tough` remains a candidate implementation and must be re-evaluated for current maintenance/API/platform fit when the updater milestone begins.

## Consequences

- Owner hubs/mirrors may distribute artifacts without becoming signing authorities.
- A rollback is performed as a new signed higher version rather than lowering version state.
- Production release operations require disciplined multi-key root custody.
- This ADR is part of the accepted Phase 0 architecture baseline.
