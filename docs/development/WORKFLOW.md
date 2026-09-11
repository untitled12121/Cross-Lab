# Development Workflow

Cross-Lab treats the Git repository as durable project memory. Chat history is working context only.

## Start of Work

Before modifying code or architecture:

1. inspect the connected repository, active branch, recent commits, and repository state;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`;
3. read `docs/development/CURRENT.md`;
4. read the active milestone document under `docs/plans/`;
5. read relevant ADRs under `docs/adr/`;
6. inspect the existing Cross-Lab implementation related to the task;
7. inspect relevant uploaded research repositories before reusing or adapting related systems;
8. reconcile documentation with actual code and verification state before continuing.

Git, code, and verification results take precedence over stale progress notes.

## Branch Policy

`main` is the canonical integrated branch and keeps its name permanently.

`planning` is reserved for planning/documentation work that is not yet integrated. Before implementation relies on a planning change, merge that change into `main` and synchronize `planning` with the integrated baseline.

Implementation work may use a short purpose-named branch when isolation is useful, for example:

```text
identity
trust-policy
protocol
simulator
quic
```

Do not create branches merely to represent phase transitions, chat sessions, temporary handoffs, or individual documentation edits. Avoid branch families such as `phase-0/...` or `foundation/phase-x-to-y`.

Never delete or rewrite a branch containing unique work until those commits are preserved in `main` or another durable ref.

## Working Style

Work in small, verifiable milestones and vertical slices.

After meaningful progress:

1. run the checks relevant to the current milestone;
2. refactor touched code where it materially improves clarity or removes duplication;
3. commit the completed change with a focused message;
4. push the durable checkpoint;
5. update `docs/development/CURRENT.md` when the resume point, verification state, blocker, or exact next task changes;
6. update the active milestone plan when milestone status changes;
7. create/update an ADR when a material architecture decision changes.

Do not create commits simply to preserve conversation, temporary notes, generated planning ceremony, or disposable files.

## Interruption and Context Limits

If context is running out, an interruption is likely, or the development environment becomes unstable, stop starting new work and preserve the current state first.

For complete work:

```text
verify -> commit -> push -> update CURRENT.md if needed -> push checkpoint
```

For valuable incomplete work that cannot be finished safely:

- preserve it on the active branch with a clearly identified checkpoint when appropriate;
- document what remains incomplete in `CURRENT.md`;
- record failed or unrun verification honestly;
- push the branch.

Never rely on `git stash` or chat history as the only copy of important work. Never discard uncommitted user work merely to make the repository appear clean.

## Architecture Changes

The Master Architecture is the primary architectural source of truth.

If implementation requires a material change to an approved trust, protocol, privilege, recovery, transport, update, compatibility, or major repository boundary:

1. stop the affected implementation work;
2. document the proposed change in an ADR;
3. obtain explicit approval;
4. update the Master Architecture and affected focused specifications in the same reviewed milestone;
5. continue implementation only after the repository reflects the approved decision.

`CURRENT.md` records execution state. It never overrides the Master Architecture or an accepted ADR.

## Research and Reuse

Uploaded/open-source repositories are research material, not automatic dependencies or architecture templates.

Before adapting source or introducing a dependency, review:

- whether Cross-Lab already contains reusable code;
- license compatibility;
- maintenance/upstream health;
- security implications;
- supported platforms;
- performance and transitive dependency cost;
- fit with Cross-Lab ownership and privilege boundaries.

Prefer maintained libraries and narrow wrappers over copying large foreign subsystems.

## Verification Baseline

Run all checks applicable to the current milestone. Once the Rust workspace exists, the normal baseline is:

```text
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Add protocol, compatibility, fuzz, networking, privilege-boundary, integration, or platform checks when the active milestone requires them.

Never mark a check as passing unless it was actually run.

Documentation-only changes still require review for broken paths, stale status, contradictory architecture statements, accidental secrets, and misleading implementation claims.

## Handoff

`docs/development/CURRENT.md` must make a fresh session self-sufficient. Keep it concise and record:

- current phase and milestone;
- canonical/active branch state;
- completed work relevant to the current milestone;
- unfinished work and blockers;
- verification status;
- active plan and relevant ADRs;
- exact next task.

Detailed completed design history belongs in Git, ADRs, focused specifications, and completed milestone records rather than accumulating in `CURRENT.md`.
