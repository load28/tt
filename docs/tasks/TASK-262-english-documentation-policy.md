# TASK-262: Require English documentation

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

Establish English as the single language for future repository documentation,
including task records.

## Scope

- Included: documentation-language contract in `AGENTS.md`, the task template,
  and task index registration
- Excluded: translating existing documentation as part of this task

## Decisions

### Decision 1: Apply the rule to all new and modified documentation

- **Context**: Task records and user-facing documentation currently mix Korean
  and English, and the contributor contract still requires parallel README
  updates.
- **Alternatives considered**: English for task records only / English for
  public documentation only / English for every new or modified document.
- **Decision and rationale**: Require English for every new or modified
  document, including task records, design notes, guides, READMEs, changelogs,
  and website copy. A single rule prevents each document category from
  developing a separate language convention.

## Work log

- 2026-08-28: Confirmed that `AGENTS.md` requires parallel English and Korean
  README updates and that the task template is written in Korean.
- 2026-08-28: Added the English-only documentation contract, removed the
  parallel Korean README requirement, and translated the task-management
  contract and task template to English.

## Issues and resolutions

None.

## Verification

- [x] `./scripts/ci agents`
- [x] `git diff --check`
- [x] Reviewed the policy and template for consistent English wording

## Result

`AGENTS.md` now requires English for all new and modified documentation. The
task template and task-management status values use the same English contract.
