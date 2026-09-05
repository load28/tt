# TASK-346: Hold the task index and the task records to the same state

- **Status**: Complete
- **Started**: 2026-09-05
- **Completed**: 2026-09-05
- **Commit**: `TASK-346: check that the task index and its records agree`

## Purpose

Three records were left reading "in progress" after their work landed, and
checking them turned up five more the other way round. `AGENTS.md` already
required the index and the record to carry the same state; nothing checked
it, so the requirement held only as long as someone remembered it — and
TASK-099 had already cleaned this up once.

## Scope

- Included: A check that every index row and the record it links agree, the
  eight records that disagreed, and the note a superseded record needs.
- Excluded: Whether a record's *content* is still accurate, and the
  numbering and template rules the index already carries.

## Decisions

### Decision 1: The rule is checked, not remembered

- **Context**: The contract is one line of `AGENTS.md`. Two of the three
  records this started from finished on the day they were written and sat
  wrong for over a week; the other five were wrong in the opposite
  direction and nobody had noticed at all. The failure is not that anyone
  was careless — it is that a written rule with no check drifts, and this
  one had already drifted once (TASK-099).
- **Alternatives considered**: Re-audit by hand periodically (what TASK-099
  did, and the drift returned); generate the index from the records (the
  index carries a title and dates a reader scans, and a generated file
  invites edits that are then overwritten).
- **Decision and rationale**: `scripts/check-task-index` reads the index and
  every record it links and reports each disagreement by name. It runs in
  the `agents` stage of both `scripts/ci` and the hosted workflow, beside
  the other entrypoint-contract checks, and changes nothing on disk.

### Decision 2: Both spellings of a state are one state

- **Context**: Records written before the English-documentation rule use
  `- **상태**: 완료`; newer ones use `- **Status**: Complete`. AGENTS.md
  says existing non-English documents are not translated unless a task
  changes them, so both will be in the tree for a long time.
- **Alternatives considered**: Translate every record now (a large diff
  touching records this task has no reason to change, against the rule
  above).
- **Decision and rationale**: The check maps both spellings of each state to
  one value, so a record is read the same way whichever language it is in.
  An unrecognised spelling is reported rather than ignored.

### Decision 3: A reversed decision is marked in its own record

- **Context**: TASK-227's Decision 1 made CI manual-only. TASK-235 restored
  the automatic triggers, and `AGENTS.md` carries the current contract — but
  TASK-227 still read as a live decision, and the stale sentence it had put
  in `CONTRIBUTING.md` ("the workflow is manual, so dispatching it runs
  these too") was still there.
- **Alternatives considered**: Delete or rewrite the superseded decision
  (the record is a chronological account of what was decided and why;
  rewriting it loses the reasoning a later reader needs to understand the
  reversal).
- **Decision and rationale**: Leave the body and put a note at the top
  naming the task that reversed it and what still stands. `AGENTS.md` now
  asks for that note as a step, so the next reversal is not left implicit.

## Work log

- 2026-09-05: Verified the three records the index still called "in
  progress" against the tree: TASK-227's own header already said complete;
  TASK-259's `EXTENSION_IDENTITY` is the upstream
  `TypeScriptTeam.native-preview` and `contentMapper.ts` no longer names the
  renamed id; TASK-260's `@next` install guidance is in the README,
  getting-started, npm README and the website, with `useTsgo` documented.
  All three had landed.
- 2026-09-05: Added `scripts/check-task-index`; its first run reported five
  further disagreements (TASK-102, 103, 108, 242, 243) — all records whose
  verification and result sections were complete and whose header was never
  flipped.
- 2026-09-05: Closed all eight, added the reversal note to TASK-227, and
  corrected the sentence it had left in `CONTRIBUTING.md`.
- 2026-09-05: Wired the check into `scripts/ci`'s `agents` stage and the
  hosted workflow's entrypoint step, and added both rules to `AGENTS.md`.

## Issues and resolutions

### Issue 1: Eight records disagreed with the index

- **Symptom**: `docs/tasks/INDEX.md` and eight `TASK-NNN-*.md` records
  carried different states — three where the index was behind, five where
  the record was.
- **Cause**: Step 3 of the task-management rules was enforced by nothing.
- **Resolution**: Decision 1, and the eight set to the state their own
  verification sections support.

### Issue 2: A title's escaped pipe broke the first parse

- **Symptom**: The check reported the state of TASK-013, 014 and 043 as
  fragments of their titles.
- **Cause**: Those titles contain `` `|>` ``, escaped as `\|` inside the
  markdown table; splitting the row on every `|` cut them in the wrong
  places.
- **Resolution**: Escaped pipes are not cell boundaries — the split ignores
  them and the title keeps its text.

## Verification

- [x] `node scripts/check-task-index` — 345 records agree; the run before
  the fixes named all eight disagreements and the three mis-parsed titles
- [x] `./scripts/ci agents`
- [x] `./scripts/ci`

## Result

Changed files: `scripts/check-task-index` (new), `scripts/ci`,
`.github/workflows/ci.yml`, `AGENTS.md`, `CONTRIBUTING.md`,
`docs/tasks/INDEX.md`, and the eight records
(TASK-102, 103, 108, 227, 242, 243, 259, 260) plus this one.
