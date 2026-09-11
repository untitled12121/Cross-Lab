# Security Policy

Cross-Lab is currently **pre-release and entering Phase 1 — Core Simulator implementation**. Phase 0 architecture and security specification is complete, but there is not yet a supported production release.

## Reporting Security Issues

Do not disclose suspected exploitable vulnerabilities in a public issue, discussion, pull request, or commit message.

Before the first public release, Cross-Lab will define a dedicated private vulnerability-reporting channel and response policy. Until that process is established, contact the repository owner privately through an available GitHub channel and disclose only the information required to establish contact.

## Security Expectations

Cross-Lab follows **Maximum Owner Control + Least-Privilege Architecture**. Contributors must preserve the security boundaries defined by the Master Architecture, including:

- transport-independent owner/device identity;
- explicit trust and authorization checks;
- capability/permission separation;
- control-plane authorization before protected data-plane use;
- isolated privileged authority;
- independent recovery authority;
- bounded protocol/resource handling;
- fail-closed behavior for ambiguous security state.

Security-sensitive changes require focused review. Changes that materially affect identity, trust, cryptographic formats, protocol compatibility, privilege boundaries, recovery, update trust, or transport/session semantics require an ADR and explicit approval before implementation.

## Secret Handling

Never commit or publish:

- private keys or recovery material;
- credentials, access tokens, API keys, or passwords;
- pairing/bootstrap secrets;
- signing keys or release credentials;
- production certificates containing private material;
- sensitive local configuration;
- captured user payloads or plaintext media/file content unless an explicitly approved test fixture requires synthetic data.

Use synthetic test data and dedicated test credentials for development. Logging and audit output must not expose secrets, private keys, credentials, reusable authentication material, or sensitive payloads.

## Research and Dependencies

External repositories and libraries must be reviewed before source adaptation or dependency adoption. Public availability is not a security or license review.

Review at least:

- license compatibility;
- upstream maintenance and release health;
- known security posture and transitive dependency surface;
- platform support;
- whether the dependency would leak authority or third-party types into Cross-Lab domain boundaries.

## Supported Versions

There are currently no supported production versions.

Security support, release-lifecycle policy, signed-update operations, and production vulnerability-reporting procedures will be finalized before the first supported release.
