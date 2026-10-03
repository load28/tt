# TASK-735: Design an authored-source service contract

- **Status**: Complete
- **Started**: 2026-10-03
- **Completed**: 2026-10-03
- **Commit**: —

> Implementation refinements are recorded in TASK-737: an equal-source contextual
> change requires a new service host, and generated probes keep distinct virtual
> identities with explicit restoration. Those decisions supersede the corresponding
> close/reopen and uniform-probe details of the initial plan.

## Purpose

Resolve the remaining installed-mapper coordinate collision from TASK-733 without
regressing contextual projections, probes, or TypeScript module identity.

## Scope

- Included: the serving-contract design and verification requirements, including
  the two macOS test portability failures blocking the local gate.
- Excluded: implementation before written design approval, upstream TypeScript
  modifications, diagnostic rewriting, and publishing before verification.

## Decisions

### Decision 1: Preserve source identity and explicitly carry projections

- **Context**: `open_served` opens lowered text under the authored URI when an
  installed mapper accepts it. TypeScript returns declaration-map targets in
  authored coordinates and direct targets in the opened text's coordinates.
  Its Location protocol cannot distinguish these after the response.
- **Alternatives considered**: Always use virtual names (changes configured
  module identity), guess from ranges (ambiguous), use a second navigation
  program (can disagree with contextual inference), or serve source buffers
  with their contextual projections at the mapper boundary.
- **Decision and rationale**: Propose the last approach. This extends the
  previously approved coordinate-provenance fix into a serving contract and
  requires architectural design approval before implementation.

## Work log

- 2026-10-03: Doctor passed. Read `open_served`, contextual snapshot publication,
  probe call sites, the content mapper protocol, and pinned TypeScript's
  `createDefinitionLocations`. Confirmed that LocationLink also does not carry
  coordinate provenance. Verified that inferred mapper contributions cannot
  override configured mappers; the proposed exchange uses the child process
  environment instead. Wrote the reviewable design below.

- 2026-10-03: User approved the written design. Wrote separate implementation
  plans for the service boundary and independently testable portability fixes;
  self-reviewed coverage, interfaces, and failure-path tests.

## Issues and resolutions

- Shared source/projection URI: implementation pending design review.
- macOS scan expectation and non-UTF-8 fixture creation: portability changes
  specified; existing failures remain recorded in TASK-732.

## Regression test (fails before the fix)

Not applicable: this stage only records the design. Implementation must retain
and extend TASK-733's failing installed-mapper reproduction before code changes.

## Verification

- [x] Existing reproduction and protocol behavior inspected
- [x] Written design approved on 2026-10-03
- [x] Implementation plan reviewed; Native execution approved 2026-10-03

## Result

The user approved the design and execution plan, then requested the repository's
task workflow as the controlling process. The design is preserved below;
TASK-736 owns portability and TASK-737 owns implementation and verification.
Separate skill-workflow documents are consolidated into these task records.

## Approved design

### Goal

Make direct and declaration-map navigation agree in an installed-mapper project,
including unsaved `.tt` and `.ttx` buffers. Preserve contextual typing, temporary
editor probes, configured module names, and the unconfigured virtual-document
path. Complete the local gates before opening the requested PR.

## Evidence and boundary

`src/engine/language/service.rs::open_served` currently opens generated text at
an authored URI if lowering it again is lossless. That makes two meanings share
one URI: a declaration map names positions in the original source, while a direct
symbol names positions in the open generated buffer. Pinned TypeScript maps
source positions before returning Location/LocationLink; neither carries the
origin needed to undo this ambiguity. Target-range heuristics cannot fix it.

Opening every projection under a virtual URI would change configured module
identity and the automatic-import behavior documented by TASK-729. Opening only
raw source without supplying the contextual projection would discard the typed
snapshot and editor recovery work. A separate navigation program would introduce
another semantic state. None of those is the proposed solution.

## Serving model

Represent a served document as an immutable revision containing its source
identity and text, generated text, mappings, anchors, recovered spans, and
revision identity. Normal contextual snapshots and temporary probes must publish
this same contract. A code string alone is insufficient for publication.

For an installed mapper, open the authored text at the authored URI. Supply its
exact contextual projection to the service's mapper through a private,
session-scoped projection exchange. The mapper uses a published projection only
when both the canonical file identity and complete input text match the revision;
an absent or stale entry uses the normal mapper compilation contract. It must
never substitute a projection for another source revision.

The exchange is an implementation detail of the TypeScript adapter. Publish
records atomically before notifying the service about a document change, keep
sessions isolated, and remove their records when the service ends. Pass the
exchange location and protocol version in the service child process environment;
its configured and inferred tt mapper processes inherit that environment.
Ordinary mapper invocations without this session environment retain their
existing behavior. No user configuration or installed package is modified.

The mapper must acknowledge the exchange protocol before the session relies on
published contextual projections. An incompatible installed mapper is an
explicit service-start failure, never silent loss of contextual typing. Exercise
this version-mismatch path in a process test. The existing `TTC_BINARY` launcher
override can select the current compiler for development fixtures; do not change
the user's global launcher configuration.

Pinned `project/session.go::SetContentMapperContributions` explicitly says
configured projects never consume contributed mappers. Therefore do not attempt
to override a configured mapper by adding an inferred-project contribution. The
inherited session environment reaches both launch paths without that assumption.

For an unconfigured service, continue opening generated text under the existing
virtual URI. This mode keeps the existing generated-coordinate protocol.

## Coordinates and editor operations

Make source versus projection coordinates a property of the served document and
request boundary. An authored-mode request supplies an authored position and
receives authored ranges; TypeScript's mapper performs the source translation.
A virtual-mode request uses projected positions and the existing inverse maps.
Route positions, target locations, edits, diagnostic ranges, completion edits,
and semantic tokens through this boundary. Do not infer the mode from whether a
range happens to map, and do not translate authored replies a second time.

Temporary probe revisions must carry explicit text and mappings, including the
mapping back to the user's buffer when the probe changes that text. Restore the
previous revision on both successful and failed requests. Contextual refreshes
must invalidate the served revision even when authored text is unchanged.
Preserve generated-binding expansion and tt-owned symbol handling explicitly;
passing an authored range to a helper expecting generated text is invalid.

## Portable verification contracts

The project scan test must compare scan results against canonical expected file
identities while retaining its separate logical source-collection assertion.

Keep the integration scenario that creates a non-UTF-8 directory entry on the
platform where that filesystem scenario is supported. Add or retain direct
path-validation tests that construct an invalid Unix `OsStr` without creating a
file, covering rejection before output publication and typed project handling.
A fixture-creation error must not become a successful assertion of compiler
behavior. Do not weaken diagnostics or silently swallow filesystem errors.

## Acceptance criteria

1. An installed-mapper regression fails on the current code and passes after the
   change. Direct and declaration-map targets agree for `.tt`, `.ttx`, a spaced
   source root, and unsaved changes. Keep a TypeScript twin where supported.
2. Configured automatic imports retain their module names. Contextual joins,
   generated/shared bindings, recovery probes, rename edits, diagnostics, and
   semantic tokens retain their existing contracts. Cover a probe failure and
   an unchanged-source contextual refresh in process/API tests where case files
   cannot observe them.
3. The two macOS verification tests exercise their stated contracts. No existing
   failure is reclassified merely to make the gate green.
4. Run the standard complete local gate and baseline ownership audit; inspect
   baseline diffs. Keep full nightly sweeps distinguished from the standard
   gate. Record any unresolved failures rather than declaring completion.

