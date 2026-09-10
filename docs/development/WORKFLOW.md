# Development Workflow

Cross-Lab treats the Git repository as durable project memory. Chat history is working context only.

## Session Start

Before modifying code or architecture:

1. Inspect the connected repository, current branch, recent commits, and working state.
2. Read `docs/architecture/MASTER-ARCHITECTURE.md`.
3. Read `docs/development/CURRENT.md`.
4. Read the active milestone record or implementation document relevant to the task.
5. Read relevant ADRs.
6. Reconcile documentation with Git, code, and verification results before continuing.

Repository state is authoritative when progress notes are stale.

## Branch Policy

Use a small purpose-based branch model.

- `main` contains integrated, reviewed project state.
- Prefer one active branch for the current unit of work.
- Name branches by purpose, for example `planning`, `foundation`, `identity`, `trust-policy`, `protocol`, `simulator`, or `quic`.
- Do not create branch names for chat sessions, temporary handoffs, or phase transitions when a purpose name is sufficient.
- Keep milestone/phase numbering in roadmap and planning documents rather than using it as a mandatory branch namespace.
- After work is integrated, retire obsolete branches only after confirming their commits are preserved in `main` or another durable ref.
- Never delete, reset, or overwrite user work merely to simplify branch history.

`docs/development/CURRENT.md` carries resume state across sessions; branch names do not.

## Working Style

Work in small, independently recoverable milestones and vertical slices. Prefer implementation progress over process artifacts: create planning documents only when they provide durable project value.

After meaningful progress:

1. Run verification relevant to the milestone.
2. Commit the completed slice with a focused message.
3. Push the branch.
4. Update `CURRENT.md` when the milestone, verification state, blocker, or exact next task changes materially.
5. Update an ADR and the Master Architecture when an approved architectural decision changes.

Do not create placeholder, temporary, or ceremonial commits when the actual change can be made directly and verified.

## Interruption and Context Limits

If context is running out, interruption is likely, or the development environment becomes unstable, stop starting new work and preserve the current state first.

For complete work:

```text
verify -> commit -> push -> update CURRENT.md when needed
```

For valuable incomplete work that cannot be finished safely:

- preserve it on the active branch;
- use a clearly identified WIP commit only when necessary;
- document what remains incomplete and which checks were not run;
- push the branch.

Do not rely on `git stash` or chat history as the only copy of important work.

## Architecture Changes

The Master Architecture is the primary architectural source of truth.

When implementation requires a material change to an approved trust, protocol, privilege, recovery, transport, update, compatibility, repository, or cross-platform contract:

1. stop the affected implementation;
2. record the decision in an ADR;
3. obtain approval where required;
4. update the Master Architecture in the same accepted change when its baseline is affected;
5. continue only after repository documentation is internally consistent.

`CURRENT.md` records progress. It does not override accepted architecture or ADRs.

## Verification

Once the Rust workspace exists, the normal baseline is:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Milestones add protocol, security, integration, fuzz, networking, privilege-boundary, or platform checks where relevant. If a command cannot be run or a feature combination makes it inappropriate, record that explicitly rather than reporting success.

Documentation-only changes are reviewed for consistency, stale references, unsupported claims, broken links, accidental secrets, and architecture/ADR contradictions.

## Session Handoff

`CURRENT.md` should remain concise and factual. It records:

- current phase and milestone;
- active branch;
- last verified checkpoint;
- completed work that matters to resumption;
- blockers or unfinished work;
- verification status;
- relevant architecture/ADRs;
- exact next task.

A fresh developer or new ChatGPT conversation must be able to resume from Git without depending on the previous chat.
