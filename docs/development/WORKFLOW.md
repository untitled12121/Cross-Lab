# Development Workflow

Cross-Lab treats the Git repository as durable project memory. Chat history is working context only.

## Start of a development session

Before modifying code or architecture:

1. Inspect the connected Cross-Lab repository and current branch.
2. Read `docs/architecture/MASTER-ARCHITECTURE.md`.
3. Read `docs/development/CURRENT.md`.
4. Read the active plan under `docs/plans/`.
5. Read relevant ADRs under `docs/adr/`.
6. Inspect recent commits and current repository state.
7. Reconcile `CURRENT.md` against code, Git history, and tests before continuing.

Never treat stale progress notes as more authoritative than the repository state.

## Working style

Work in small, independently recoverable milestones and vertical slices. Avoid accumulating large unverified changes.

After meaningful progress:

1. Run the verification relevant to the current milestone.
2. Commit completed work with a focused commit message.
3. Push the branch or commit to the connected GitHub repository.
4. Update `docs/development/CURRENT.md` with the current milestone, completed work, unfinished work, verification status, and exact next task.
5. Update the active plan when task or milestone status changes.
6. Create or update an ADR when a material architecture decision changes.
7. Verify the remote repository contains the expected checkpoint.

Do not mark verification as passing unless it was actually run.

## Interruption and context limits

If the conversation becomes long, context is running out, an interruption is likely, or a development environment becomes unstable, stop starting new work and create a checkpoint first.

For complete work:

```text
verify -> commit -> push -> update CURRENT.md -> push checkpoint
```

For valuable incomplete work that cannot be finished safely:

- preserve it on the active feature branch;
- use a clearly identified WIP commit when appropriate;
- document exactly what remains incomplete in `CURRENT.md`;
- record failed or unrun verification honestly;
- push the branch.

Do not use `git stash` or chat history as the only copy of important work.

Never discard, reset, rewrite, or overwrite uncommitted user work merely to make a checkpoint look clean.

## Architecture changes

The Master Architecture is the primary architectural source of truth.

If implementation requires a material change to an approved boundary or contract:

1. stop the affected implementation work;
2. document the proposed change;
3. create or update an ADR;
4. obtain approval where required by the architecture policy;
5. update the Master Architecture if the approved decision changes it;
6. continue implementation only after the repository reflects the approved design.

`CURRENT.md` records progress. It does not override the Master Architecture or accepted ADRs.

## Verification baseline

Before a milestone is considered complete, run all checks applicable to it. Once the Rust workspace exists, the normal baseline is expected to include:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Additional protocol, security, integration, fuzz, networking, or platform checks are required when the milestone calls for them.

Documentation-only milestones must still be reviewed for internal consistency, unsupported claims, stale links, and accidental secret material.

## Session handoff

At the end of a substantial session, ensure `CURRENT.md` records:

- current phase and milestone;
- active branch;
- last verified commit;
- completed work;
- unfinished work;
- verification status;
- known failures or blockers;
- active plan and relevant ADRs;
- exact next task.

The repository must be sufficient for another developer or a fresh ChatGPT conversation to resume without relying on the previous chat.
