## What this changes

Describe what the change intends to do, and link its task record (`docs/tasks/TASK-NNN-*.md`).

## Checklist

See "Housekeeping" in [CONTRIBUTING.md](../CONTRIBUTING.md).

- [ ] At least one test fails without this change's non-test code (for a bug fix, usually one case file under `tests/cases/`), and the task record's "Regression test (fails before the fix)" section names it and the failure it reported. If no test can, the record says `Not applicable:` and why.
- [ ] The tests include reasonable permutations of the fixed input.
- [ ] Every baseline change (`tests/baselines/reference/`, `tests/fixtures/**/expected.*`) is committed with this change, and I read its diff.
- [ ] `./scripts/ci` passes locally.
