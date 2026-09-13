---
title: Branch Strategy
nav_order: 4
---

# Branch Strategy

This repository uses short-lived branches and pull requests so `main` stays deployable and easy to review.

## Branch Types

| Branch | Use |
|--------|-----|
| `main` | Stable integration branch. Merge completed work here through pull requests. |
| `feature/...` | New product work, UX improvements, refactors, and milestone tasks. |
| `fix/...` | Bug fixes that are narrower than a larger feature branch. |
| `milestone/...` | Temporary coordination branches when a milestone needs several related commits before review. |

## Standard Workflow

1. Start from an up-to-date `main`.
2. Create a focused branch, usually `feature/...` or `fix/...`.
3. Keep changes scoped to one milestone or problem.
4. Run the relevant automated and manual validation for the work.
5. Open a pull request with a short summary, validation notes, and any follow-up items.
6. Squash merge the pull request into `main`.

## Pull Request Expectations

- Explain why the change exists, not just which files changed.
- Link the milestone or planning document when the work follows a tracked phase.
- Record the validation you ran, for example `./run-tests.sh`, Safari checklist results, or UAT notes.
- Call out any intentional gaps, deferred work, or manual validation that still needs to happen.

## Merge Policy

- Prefer squash merges so `main` keeps one clean commit per reviewed change.
- Keep commit history on the branch as needed during development; the pull request is the review unit.
- Do not merge directly into `main` from a local working tree.

Recent examples:
- PR `#30` merged `feature/phase2-cleanup-and-operator-polish` with a squash merge.
- PR `#31` merged `feature/phase2-correction-workflow-polish` with a squash merge.

## Branch Cleanup

- Delete feature branches after the pull request is merged.
- Sync local `main` with `origin/main` before starting the next task.
- If follow-up work appears during review, open a new branch instead of reusing an already merged one.

## Validation Reminder

Changes that affect operator-facing behavior should update the relevant validation artifacts in `docs/validation/` along with the code. See `docs/validation/VALIDATION_WORKFLOW.md` for the expected validation flow.
