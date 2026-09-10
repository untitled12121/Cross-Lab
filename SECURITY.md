# Security Policy

Cross-Lab is currently in **Phase 0 — Architecture and Security Specification** and has not published a production release.

## Reporting security issues

Do not disclose suspected exploitable vulnerabilities in a public issue, discussion, pull request, or commit message.

Before the first public release, Cross-Lab will define a dedicated private vulnerability-reporting channel and response policy. Until that process is established, contact the repository owner privately through an available GitHub channel and disclose only the information required to establish contact.

## Security expectations

Cross-Lab follows **Maximum Owner Control + Least-Privilege Architecture**. Contributors must preserve the security boundaries defined by the Master Architecture, including transport-independent identity, explicit authorization, control/data-plane separation, isolated privileged authority, and independent recovery authority.

Security-sensitive changes require focused review. Changes that materially affect identity, trust, cryptographic formats, protocol compatibility, privilege boundaries, recovery, update trust, or transport/session semantics require an ADR and approval before implementation.

## Secret handling

Never commit or publish:

- private keys or recovery material;
- credentials, access tokens, API keys, or passwords;
- signing keys or release credentials;
- production certificates containing private material;
- sensitive local configuration or captured user payloads.

Use synthetic test data and dedicated test credentials for development. Logging must not expose secrets, private keys, credentials, or sensitive payloads.

## Supported versions

There are no supported production versions yet. Security support and release-lifecycle policy will be defined before the first production release.
