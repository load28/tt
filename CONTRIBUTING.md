# 기여 가이드

## 개발 환경

먼저 개발 환경을 진단합니다.

```sh
./scripts/doctor
```

- Rust 버전: `rust-toolchain.toml`
- Node.js 의존성: `npm ci`
- Bun: required by the npm integration and website gates
- 전체 상태 확인: `./scripts/doctor`

## 로컬 개발 환경 (`scripts/setup`)

로컬 컴파일러와 VS Code 확장을 준비합니다.

```sh
npm ci             # TypeScript 7 포함 — package.json이 버전을 고정한다
./scripts/setup    # release ttc + VSCode 확장
```

`npm ci`는 `package.json`의 TypeScript를 설치합니다. `scripts/setup`은 release
`ttc`와 VS Code 확장을 빌드하고 확장을 설치합니다.

테스트 프로젝트에서는 TT 전용 명령 없이 일반 패키지 매니저로 설치합니다:

```sh
pnpm add -D file:/path/to/tt/npm/tt-lang   # 재빌드 후에는 --force로 재설치
pnpm add -D typescript@7.1.0-dev.20260826.1
pnpm ttc --check-types src
```

launcher(`npm/tt-lang/bin/ttc.js`)는 이 저장소의 `target/release/ttc`를
실행합니다.

## 절대 불변 원칙

어떤 변경도 이 세 계약을 깨뜨릴 수 없습니다 (자세한 내용은 [`AGENTS.md`](./AGENTS.md)):

1. 모든 유효한 TypeScript 파일은 그대로 유효한 `.tt` 파일이다 (바이트 단위 통과).
2. tt 수준 에러는 ttc가 직접 보고하고, 방출 코드는 타입 트릭 없는 순수
   TypeScript다 — ttc가 방출한 코드가 tsc 에러를 만들면 안 된다.
3. 해결은 책임 있는 컴파일러 계층에 일반화해 구현한다 — 특정 테스트나 문자열
   모양을 겨냥한 분기·휴리스틱·진단 억제·폴백으로 덮지 않는다.

## 작업 절차 (필수)

모든 작업은 태스크 문서로 관리됩니다:

1. `docs/tasks/INDEX.md`에서 다음 번호를 확인하고 `docs/tasks/TEMPLATE.md`로
   태스크 문서를 만든 뒤 INDEX에 등록합니다.
2. 작업 중 결정·문제·범위 변경을 태스크 문서에 기록합니다.
3. 완료 시 검증 결과를 기록하고 상태를 갱신합니다.
4. 커밋 메시지는 태스크 ID로 시작합니다: `TASK-012: ...`.

## Housekeeping

A pull request should:

- describe what the change intends to do;
- include at least one test that fails without the change's non-test code.
  For a bug fix that is usually one case file under `tests/cases/` (see
  "Adding a test case"). Run it against the unfixed code and record its
  path and the failure it reported in the task record's "Regression test
  (fails before the fix)" section, which `scripts/check-task-index`
  requires from TASK-636 on; a change that fixes no bug says
  `Not applicable:` and why there;
- include reasonable permutations of the fixed input, not only the reported
  one;
- include the baseline changes it causes, in the same commit, after reading
  their diff (`git diff -- tests/baselines tests/fixtures`). A baseline that
  changed for no reason the change explains is a clue about something it
  did not intend;
- pass `./scripts/ci` locally.

The pull request template (`.github/pull_request_template.md`) repeats this
as a checklist. These rules follow the "Housekeeping" section of
TypeScript's `CONTRIBUTING.md`.

## 머지 전 검증 게이트

```sh
./scripts/ci
```

GitHub Actions의 [`CI`](./.github/workflows/ci.yml)는 `main`과 `release-X.Y`의
push·PR에서 자동으로 돕니다. `scripts/ci`는 같은 핵심 게이트를 로컬에서 재현하며,
PR을 열기 전에 먼저 실행해야 합니다.

| 단계 | 내용 |
| --- | --- |
| `agents` | 에이전트 진입점 계약(`CLAUDE.md`, `scripts/doctor`) |
| `rust` | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, unused baselines (`scripts/check-baselines`) |
| `npm` | npm 릴리스 도구, 프로젝트 초기화기, unplugin 어댑터, 심의 도구 테스트 |
| `website` | 공개 사이트의 타입 검사와 정적 렌더 (Bun 필요) |
| `native` | TypeScript 7을 실제로 구동하는 타입 검사 모드 |
| `extension` | VS Code 확장 빌드와 서버 테스트 |

단계를 골라 돌릴 수 있습니다.

```sh
./scripts/ci rust              # Rust 게이트만
./scripts/ci --skip extension  # 확장 테스트만 빼고
./scripts/ci --list            # 단계 이름
```

Every suite that type-checks runs the TypeScript that `package.json` pins,
installed by `npm ci`: the typed suites drive it through ttc, and the
integration tests run its own `tsc` rather than one on `PATH`. An installed
version other than the pin fails the suites. Without that install or
rolldown, the related tests **skip rather than fail.** `scripts/ci` warns at
startup about what is missing and which checks disappear, and repeats it in
the summary — a pass with warnings is not the pass CI gives. Only the
`native` stage fails instead of warning, because running the TypeScript 7
path is the reason that stage exists.

호스팅된 실행을 다시 확인해야 하면 Actions 탭에서 `CI` → **Run workflow**로도
시작할 수 있습니다.
새 기능에는 반드시 테스트를 추가하세요:

- A bug fix → one case file under `tests/cases/` (see "Adding a test case"
  below)
- Emitted output, diagnostics, type-check results, runtime behaviour → case
  files under `tests/cases/` (see "Adding a test case" below)
- A library API other than the emission, non-default `Options` →
  `tests/compile.rs`; the CLI, files, or processes → `tests/integration.rs`
  or `tests/cli*.rs`
- TS 통과 계약 → `tests/passthrough.rs`
- **산출물 전체**가 계약인 것(방출된 TypeScript, 렌더된 진단) →
  `tests/fixtures/` 스냅샷. 픽스처 디렉터리에 `input.tt`를 넣고
  `UPDATE_EXPECT=1 cargo test --test snapshot`으로 기대 파일을 만든 뒤 **그 diff를
  읽으세요** — 그 diff가 리뷰 대상입니다. 부분 문자열 어서션은 여분의 문장이나
  어긋난 들여쓰기를 잡지 못합니다.

  Generate and check emit fixtures in a checkout configured with `npm ci`.
  TypeScript supplies annotations for generated match/result storage, so these
  fixtures pin the annotated artifact. Without a toolchain, emit comparisons
  skip and `UPDATE_EXPECT=1` refuses to overwrite the configured expectations.
  The gate sets `TTC_REQUIRE_TSGO=1` so a missing prerequisite is a failure.
  Compiler-owned standard-library modules are provided to contextual analysis
  in memory; running an editor or installing a generated `@tt/std` package is
  not a fixture prerequisite.

### Adding a test case

The default regression test for a bug fix is one case file. Add a `.tt` or
`.ttx` file under `tests/cases/compiler/` with the code that shows the bug is
fixed, or under `tests/cases/conformance/<feature>/` when it pins one area of
the language. This follows TypeScript's own "Adding a Test" rule
(`tests/cases/compiler`, `tests/cases/conformance`, and
`tests/baselines/reference` in microsoft/TypeScript).

Case files take metadata lines in the form `// @name: value`:

- `// @filename: <path>` starts a new compilation unit, so one case can hold
  several `.tt`, `.ttx`, `.ts`, `.tsx`, or `tsconfig.json` files that import
  each other. Only comments may appear before the first one. A case without
  it is one unit named after the case file. Without a `tsconfig.json` unit,
  the case gets a strict ES2022 bundler configuration.
- `// @rewriteImports: js|ts|off` and `// @noVerify: true|false` are ttc's
  `--rewrite-imports` and `--no-verify`. There are no other options; an
  unknown directive or value fails the case.
- A comma-separated value runs the case once per value, as TypeScript's
  compiler runner does (`varyBy`): `// @rewriteImports: js, ts, off` writes
  three sets of baselines, named `<name>(rewriteimports=js).<kind>` and so
  on, with every varied option in the name, sorted. `*` stands for every
  value of the option and `-value` (or `!value`) removes one. Two varied
  options run every combination, at most 25.
- `// @run: <unit>` executes the case: a `.tt`, `.ttx`, `.ts`, `.tsx`,
  `.mts`, or `.cts` unit named as in its `// @filename` (the case file's
  own name when it has none) is the entry. Every configuration of the case
  runs, so a varied case gets one `.stdout` per configuration. A `.ttx` or
  `.tsx` entry needs a `tsconfig.json` unit whose `jsx` emits JavaScript
  (`react` with a `jsxFactory` the case defines, for instance).
- `// @twin: <unit>` names a TypeScript unit that is the `@run` entry's
  twin: the same program written by hand in plain TypeScript. It runs after
  the entry, from the same emitted tree, and the case fails when its stdout
  or exit status differs from the entry's, or when it prints nothing.
- `// @expectErrors: <code>, ...` names tt diagnostics (`try-placement`,
  `match-placement`, ...) the case must report as `error[<code>]`; the case
  fails when it compiles cleanly or leaves one out. It takes no `@run`.
- `// @baselines: <kind>, ...` keeps only the listed baselines (`ts`,
  `errors.txt`, `map.txt`, `types`, `stdout`; `stdout` includes `stderr`).
  The default is every kind.
- `// @origin: tests/<file>.rs::<test>` names the inline Rust test the case
  was converted from (see "Converting an inline test" below). It changes
  nothing about how the case runs; a case converted from several tests
  carries one line per test.

A case with `@twin` or `@expectErrors` carries its own oracle. When the
oracle disagrees, the case fails unless `tests/oracle-failures.txt` lists
it with what the run observes and the task that tracks the defect (one line
per case: the case name, a tab, the observation the failure prints, a tab,
`TASK-NNN: ...`). A case name there may contain `*`, which stands for any
text, so one line lists a defect every position of a generated example
shows: it excuses each case it names that observes what it says, and a run
of every case it names fails when none does. A listed case whose oracle agrees fails too, and so does a line that
names no case, so the list is always the exact set of known failures.

Case names must be distinct across `tests/cases`, because each case writes
its baselines as `tests/baselines/reference/<name>.<kind>`:

| Baseline | Contents |
| --- | --- |
| `<name>.ts` | every unit, then every file `ttc --out-dir` wrote (support modules by name only); a written file that does not end in a newline is followed by `\ No newline at end of file` |
| `<name>.errors.txt` | what `ttc --out-dir` and `ttc --check-types` report, then what `tsc` reports on the emitted TypeScript; absent when all three succeed |
| `<name>.map.txt` | the source-to-output mappings of the editor projection (`ttc::emit_mapped`) |
| `<name>.types` | the engine's hover for each classified identifier, under its source line |
| `<name>.stdout` | with `@run`: what the program printed to stdout |
| `<name>.stderr` | with `@run`: the exit status and stderr when the program failed, printed to stderr, timed out, or was not run; absent otherwise |

A runtime baseline is the default regression test for a fix whose bug is
what the emitted program does (evaluation order, a value evaluated twice,
`this`, short-circuiting, disposal order) rather than what ttc reports.
Write the case as a program that prints what it observed, as TypeScript's
evaluation tests (`src/testRunner/unittests/evaluation/` in
microsoft/TypeScript) push to an `output` array: log each side effect, and
print values with `JSON.stringify` or template strings, whose text does not
depend on the Node.js version. The case runs only when it compiles cleanly
(no `.errors.txt`); otherwise `.stderr` says it was not run. The emitted
tree is compiled to JavaScript by the pinned `tsc` with the case's own
`tsconfig.json` (plus `--noEmit false`, `--rewriteRelativeImportExtensions`,
and an output directory) and run as an ES module by `node` under the
permission model (`--permission` with read access to that directory only),
with an empty environment apart from `PATH` and `TZ=UTC`, stdin closed, and
a 10-second timeout. `console` needs the `dom` library, which the default
configuration includes; a case with its own `lib` lists it. stderr is kept
without Node.js's own stack frames (`node:internal`) and version line.

Create or refresh the baselines, then read the diff before committing it with
the change:

```sh
UPDATE_EXPECT=1 cargo test --test case_baselines
git diff -- tests/baselines
TT_CASES=<name fragment> cargo test --test case_baselines   # a few cases while iterating
```

The `.ts`, `.errors.txt`, `.types`, `.stdout`, and `.stderr` baselines need
the pinned TypeScript (`npm ci`), and the runtime baselines a Node.js 22
recent enough for `--permission` (22.13 or later). Without TypeScript they
are skipped, `TTC_REQUIRE_TSGO=1` turns the skip into a failure, and
`UPDATE_EXPECT=1` refuses to run.

A baseline holds the text it pins byte for byte: only the case's temporary
directory is replaced by `$DIR` (and, on Windows, `\r\n` and `\` by `\n` and
`/`), so a line ending, an escape sequence, or a missing final newline in
the output is part of what the case asserts.

### Converting an inline test

Most of the older tests are Rust functions in `tests/compile/*.rs` and
`tests/integration/*.rs` that compile a string and assert on part of the
answer. `scripts/convert-inline-tests` turns the ones whose observations a
case pins into case files, and proves each conversion before the Rust test
is removed:

```sh
scripts/convert-inline-tests scan compile        # or integration
scripts/convert-inline-tests harness compile     # the bodies around recording helpers
scripts/convert-inline-tests record compile      # every program a helper compiled
scripts/convert-inline-tests generate compile    # one case per program, with @origin
UPDATE_EXPECT=1 cargo test --test case_baselines # their baselines; read them
scripts/convert-inline-tests prove compile       # the bodies against the baselines
scripts/convert-inline-tests apply compile       # delete what was proven
scripts/convert-inline-tests clean compile
```

The harness copies each test body verbatim around helpers of the same
names: `ok`, `ok_tsx`, `err`, `advice`, and `codes` for the compile suite,
`run`, `run_with_std`, `typecheck`, and `typecheck_with_std` for the
integration suite. Recording, they call the library and log the program;
proving, they answer from the case's baselines (the emitted file of the
`.ts` baseline, the diagnostics and their `help:` lines of the
`.errors.txt` baseline's `ttc --out-dir` section, its `tsc` section, the
`.stdout` baseline). A test is removed only when every assertion of its
body holds against those answers. A body that reaches anything else (the
library directly, its own files or processes, a helper that does) does not
build in the harness and stays in Rust, and so does a test whose
observation a baseline does not hold, which the proof reports as a failed
assertion.

### The case matrix

`tests/cases/conformance/matrix/` holds cases generated from a spec, the
way TypeScript's conformance suite covers each feature in many contexts:
`scripts/generate-cases` reads `tests/matrix/` and writes one case per
combination of a construct's form (a `match` with tag, literal, tuple, or
`is` patterns, a `try`, a `result` block, a let-else, an `if let`, a
pipeline, a `flow`, `val`, a `variant` declaration, ...), a host position
(a declaration initializer, a call argument, a template literal, a class
field, a loop head, a match arm, a result block's body, ...), and a
companion feature written around it (`await`, `yield`, an optional chain,
a spread, a throw caught around it, `using`, `finally`). Constructs that
opt in also run in `.ttx` programs, in JSX positions (an attribute, a
child, `&&` and `? :` conditional rendering, a fragment, a spread
attribute, a component's prop, a nested element) and in a function
component's body, against `.tsx` twins; their `tsconfig.json` compiles JSX
with the harness's `element` factory, which renders an element to a
string.
Every form meets every position, and the companions are chosen so that
every pair of the three factors occurs at least once (all-pairs testing,
NIST SP 800-142).

A combination the documented placement rules reject (`docs/ai/tt.md`,
`docs/design/try-result-scopes.md`) becomes a case with `@expectErrors`
and only an `.errors.txt` baseline. Every other combination runs against a
`twin.ts` generated from the same spec: each form's twin is written from
the construct's documented semantics (a `match` as a chain of tests over a
temporary, a `try` as an `unwrap` that throws its failure to the function it
leaves, a `result` block as a function run on the spot), never from ttc's
output. These cases keep only `.stdout` (and `.errors.txt`, which should be
absent): the twin is the oracle, and the emitted TypeScript, hovers, and
mappings of thousands of near-identical programs would bury a reviewer
without testing more.

Edit the spec, not the cases, and regenerate:

```sh
node scripts/generate-cases            # rewrite the matrix, removing stale cases
node scripts/generate-cases --stats    # the counts per construct
UPDATE_EXPECT=1 TT_MATRIX_CASES=all cargo test --test case_baselines
```

`tests/case_baselines.rs` fails when the committed cases differ from what
the spec generates. A pull request runs a fixed-seed sample of the matrix
(`TT_MATRIX_CASES=<count>` and `TT_MATRIX_SEED=<number>` choose another);
the nightly `exhaustive` job runs every case with `TT_MATRIX_CASES=all`, as
`TT_CASES=<fragment>` also runs every matching case. The baselines of the
cases a run did not sample are recorded as not sampled, so
`scripts/check-baselines` neither judges nor flags them. The fuzz
regressions, the incremental suite, and the fuzz seed corpus leave the
matrix out: its programs are one spec's repetitions, and it has its own
runner.

The same rows also become editor cases under
`tests/cases/editor/matrix/<construct>/`, for the constructs and host positions
`tests/matrix/editor.mjs` lists: the case's program with fourslash
markers, and a `.ts`/`.tsx` twin with the same markers at the
corresponding points. A spec template marks a point with `/*@name*/`
(stripped from the compiled cases); the name's leading lowercase letters
are its kind, and `tests/matrix/editor.mjs` says which verbs each kind
asks (`bind`: a name a pattern or declaration introduces; `use`: a read of
one; `call`, `arg`, `member`, `field`, `attr`, and `operand`, the input
each companion feeds the construct). Every case also asks for semantic
tokens and for what the VS Code adapter publishes. A form's twin is its
`ts` template unless it has an `edit` template: the twin an editor is held
to must declare what the tt program binds (`const { r } = s` for
`Circle(r)`), which a runtime twin that reads `s.r` does not, and
`tests/matrix/editor-harness.ts` types the harness's `unwrap` as a `try`
is typed. Where the twin cannot answer as tt promises, a construct or
position withholds that verb (`editorWithholds`), and the task that adds
it says why. The generator fails when a marker is on one side only. The
twin's answers are the oracle, after the normalizations "Adding an editor
case" describes; a generated case keeps a baseline only when it differs
from its twin, holding just the differing questions, and every difference
is listed in `tests/editor-matrix-differences.txt` as `by-design` (citing
where `docs/ai/tt.md` documents it) or `defect` (with the task that
records it). A case name there may contain `*`. The suite fails on a
difference the file does not list, on a listed question that agrees with
its twin, and, unfiltered, on a line that names no question. A pull
request runs a fixed-seed sample of the editor matrix too, and the nightly
job runs all of it with `TT_MATRIX_CASES=all`.

```sh
node scripts/generate-cases                        # also rewrites tests/cases/editor/matrix
UPDATE_EXPECT=1 TT_MATRIX_CASES=all cargo test --test editor_cases
```

### The diagnostics matrix

`tests/cases/conformance/diagnostics/<code>/` holds cases generated from
`tests/matrix/diagnostics.mjs`, one directory for every tt diagnostic code
`ttc explain` lists. Each entry gives a code's examples: a minimal invalid
program and the nearest valid one, the fix the code's explanation
suggests. An example of a construct (a `match`, a `result` block, a
pipeline, a let-else, ...) is placed in every host position of
`tests/matrix/positions.mjs` where the rule applies, `.ttx` positions
included: where the construct is accepted for a rule about the construct
itself, and where the position rejects it (`rejects`) for a placement rule,
together with the hosts `extraPositions` adds (an enum member, a computed
member name, a decorator, the heritage of a decorated class, a later
declarator of a `for` head). A whole-program example (`kind: "module"`) is
one case per surface.

An invalid case carries `// @expectDiagnostic: <code>` and marks the range
the diagnostic must cover with `[|...|]` (the markers are removed before
the case is compiled). The case passes when `ttc --out-dir`, `ttc
--check-types`, and `ttc --server`'s `check` and `typedCheck` each report
exactly the marked ranges with that code and no other tt diagnostic; the
two command-line reports are compared by where they start, the server's by
their whole range. `// @typedOnly: true` names a rule only the checker can
decide, which the untyped surfaces must not report. The fixed case runs
against a TypeScript twin as the case matrix does (a placement rule's fix
lifts the construct into a declaration before its host, or names its own
form in `fixes`); a fixed program that does not run carries `// @expectClean:
true` and must compile cleanly. These cases keep only `.errors.txt`.

`ttc explain`'s examples are tested too, as rustc tests the examples of
its error-code explanations: every block indented by four spaces in an
explanation is held by a case with `// @explains: <code> <n>` (its `n`th
block), which reproduces the code or compiles cleanly, and whose lines
contain the block's lines in order. Every code's explanation has an
example that reproduces it. A code no program can report is listed in
`tests/diagnostic-codes-without-cases.txt` with the reason and the task
that records it; every other code without a case fails the suite, and so
does a listed code that has one.

A pull request runs every explanation case and one invalid and one fixed
case of each code, chosen by a fixed seed (`TT_MATRIX_SEED` chooses
another); the nightly `exhaustive` job runs all of them with
`TT_MATRIX_CASES=all`. A disagreement is listed in
`tests/oracle-failures.txt` like the case matrix's.

```sh
node scripts/generate-cases --stats    # also counts the cases of each code
UPDATE_EXPECT=1 TT_MATRIX_CASES=all cargo test --test case_baselines
TT_CASES=match-duplicate-arm cargo test --test case_baselines   # one code
```

### Adding an editor case

An editor behaviour is pinned by one file under `tests/cases/editor/`, in the
spirit of TypeScript's fourslash tests. The file is a `.tt` or `.ttx` source
(or several `// @filename:` units) with named markers `/*name*/` and ranges
`[|text|]`, both removed before the file is written, and verb lines that say
what to ask where:

```ts
// @hover: use
// @completions: body
// @references: binding
// @semanticTokens: *
export variant Shape { Circle([|radius|]: number), Point }
export const area = (s: Shape) => match (s) {
  Circle([|/*binding*/radius|]) => /*use*/[|radius|] * 2,
  Point => /*body*/0,
};
```

The verbs are `hover`, `completions`, `definition`, `references`, `rename`
and `signatureHelp` (followed by marker names) and `semanticTokens` and
`diagnostics` (followed by unit names, or `*` for every `.tt`/`.ttx` unit).
`tests/editor_cases.rs` asks each question through the engine API and
through `ttc --server`, fails when the two answers differ, and writes the
answer to `tests/baselines/reference/editor/<name>.baseline`. When a
`references` or `rename` marker sits inside a range, the answer must be
exactly the case's ranges.

A completion entry that imports its name from a module (an auto-import,
shown with `from "..."`) is also resolved through both transports, and the
edits accepting it makes are shown under it (`resolve <label> from ...`).

`diagnostics` and `completions` are also asked of the VS Code adapter
(`editors/vscode/server/out/server.js`, over LSP), because what an editor
shows is what the adapter makes of the engine's answers. A `diagnostics`
section adds `published:`, the list the adapter publishes for the unit
after merging the text, typed, service, and hint layers, with each entry's
source (`ttc`, `ts`, or `tt`). A `completions` section adds `editor
completion:`, the adapter's items with their LSP 3.17 `CompletionItemKind`
names and `CompletionItemTag`s. The adapter must be built (`npm ci --prefix
editors/vscode && npm --prefix editors/vscode run compile`); without it the
suite skips, and `TT_REQUIRE_EXTENSION=1` turns the skip into a failure.

`// @expectDiagnostic: <code>` (with `// @typedOnly: true` for a rule only
the checker decides) holds one diagnostic to every surface that shows it.
The tt diagnostics the adapter publishes must be exactly the case's
`[|ranges|]`, each with that code, severity Error, and no tags; and `ttc
--check` (unless `@typedOnly`) and `ttc --check-types`, run in the case's
project, must each report the same number of tt diagnostics in the unit,
every one with the published code, message, and start, the published width
when the range is on one line, and, as its labels, the published related
information. For a code whose report restates TypeScript's own syntax
verdict (`verify-failed`, `source-not-typescript`), the adapter publishes
TypeScript's diagnostic in its place, which must be at the range, and the
command line must report the code where it starts. A difference is a defect, listed in
`tests/editor-diagnostic-differences.txt` (the case name, a tab, the first
line of the failure, a tab, and the task); the suite fails on a difference
the file does not list and on a listed one that no longer occurs.
`scripts/generate-cases` writes one such case per code under
`tests/cases/editor/diagnostics/` from the first example of
`tests/matrix/diagnostics.mjs` (see "The diagnostics matrix"), in its first
`.tt` position and, when it has one, its first `.ttx` position.

A TypeScript twin, the same name with `.ts` or `.tsx` and the same units
with `.ts`/`.tsx` for `.tt`/`.ttx`, is asked the same questions at the same
markers through `tsgo --lsp`; a unit that is not `.tt`/`.ttx` may be left
out of the twin, which then shares the case's. Answers are compared as
`parity_view` in `tests/editor_cases.rs` reduces them: a location is its
unit's stem, the marker it starts at if any, and the text it covers; a
completion list is its labels and kinds, without the labels a
`// @parityIgnores: <labels>` line in the case names (a name only one
side declares, such as a twin's temporary). When the twin's text is the
source's, semantic tokens and diagnostics are compared whole; otherwise
the tokens are compared at each marker both files have, and diagnostics by
location instead of coordinates. The diagnostics compared are the
published list. Every answer that differs from TypeScript's is
shown in the case's baseline and listed in
`tests/baselines/reference/editor/failingParity.txt`, which is a baseline
too: a new difference and a fixed one both change it. (The generated
editor cases of "The case matrix" keep their own list.)

```sh
UPDATE_EXPECT=1 cargo test --test editor_cases
git diff -- tests/baselines/reference/editor
TT_CASES=<name fragment> cargo test --test editor_cases   # a few cases while iterating
```

### Managing the baselines

Every reference file is a baseline some test compares: everything under
`tests/baselines/reference/` (the editor cases own `editor/`, `tests/public_api.rs` owns `api/`), and the `expected.*` files under
`tests/fixtures/emit/`, `tests/fixtures/diagnostic/`, and
`tests/fixtures/practical-diagnostics/`. A test fails on a **missing**
baseline and on a **modified** one, with the diff and the
`UPDATE_EXPECT=1 cargo test --test <suite>` command that regenerates it.

An **unused** baseline, one no test compared, fails too. Each comparison
records its path when `TT_BASELINE_TRACKING_DIR` is set, and
`scripts/check-baselines` compares the recorded paths with the files on
disk. It only judges a suite that ran unfiltered (no test name, `--skip`, or
`TT_CASES`), so a filtered run is reported as incomplete rather than
flagging the baselines it did not reach.

A failing comparison leaves the committed baseline alone and writes what the
run produced to `tests/baselines/local/` (ignored by git), as TypeScript's
runner does: a baseline under `tests/baselines/reference/` at the same
relative path, a fixture's `expected.*` at its repository path
(`tests/baselines/local/tests/fixtures/...`), and an empty `<path>.delete`
marker for a baseline that should no longer exist, stale or unused. A
comparison that matches removes its local file.

```sh
scripts/baseline-diff                          # the last run's new baselines against the committed ones (DIFF=<tool> to use your own)
scripts/baseline-accept                        # copy them over the committed ones, apply the .delete markers, empty tests/baselines/local
node scripts/check-baselines --run             # the baseline suites from a clean tests/baselines/local, then the unused check
node scripts/check-baselines --run --accept    # the same, then scripts/baseline-accept
git diff -- tests/baselines tests/fixtures     # review before committing
```

`UPDATE_EXPECT=1 cargo test --test <suite>` still writes the committed
baselines directly; it is the one-suite shortcut for the run-and-accept
cycle.

`./scripts/ci rust` clears `tests/baselines/local/`, runs `cargo test` with
tracking, and then the check. The hosted `CI` does the same, then runs
`node scripts/check-baselines --ci`: the baseline suites again, the unused
check's `.delete` markers, and `scripts/baseline-accept`; it fails when the
tree then differs from the commit, listing missing, modified, and unused
baselines and uploading the difference as the `fix_baselines.patch`
artifact. `git apply fix_baselines.patch` reproduces it locally.

### The public surface

`tests/public_api.rs` holds three surfaces to baselines under
`tests/baselines/reference/api/`, the way TypeScript holds its API to
`tests/baselines/reference/api/typescript.d.ts`:

- `ttc.api.txt`: the library's public API as rustdoc renders it on the
  pinned toolchain (`cargo doc --no-deps --lib`): every item's declaration,
  inherent methods, trait implementations, and auto-trait implementations.
- `server-protocol.txt`: the `ttc --server` protocol. One example request
  per method goes to a server over a small project; the baseline shows the
  shape of each request and answer (keys and JSON types). The test fails
  when `src/server.rs` dispatches a method with no example or reads a
  parameter no example sends.
- `lsp-capabilities.json`: the `initialize` result of the VS Code
  extension's language server. It needs the server built
  (`npm ci --prefix editors/vscode && npm --prefix editors/vscode run
  compile`); without it the test skips, and `TT_REQUIRE_EXTENSION=1`, which
  `./scripts/ci rust` and CI set, makes the skip a failure.

A change to any of them is a surface change: read the diff, and regenerate
with `UPDATE_EXPECT=1 cargo test --test public_api`.

### TypeScript's own test cases

Contract 1 is checked against TypeScript's test suite. `tests/typescript-cases.json`
pins the microsoft/TypeScript commit the pinned `typescript` package was
built from (its `gitHead`) and the tree ids of `tests/cases/compiler`,
`tests/cases/conformance`, and the fourslash tests there; `scripts/fetch-typescript-cases` makes a
sparse, blobless checkout of exactly those into `target/typescript-cases/`
(about 55 MB, a few seconds) and verifies the tree ids. Nothing from it is
committed; the checkout keeps TypeScript's `LICENSE.txt` and `NOTICE.txt`
(Apache-2.0).

`tests/corpus.rs` splits each case into units by `// @filename`, asks the
pinned `tsc` which `.ts`/`.tsx` units parse (and have no TS1xxx grammar
error), and requires each of those to come back from `ttc` byte for byte
with no diagnostic. A unit that does not is listed in
`tests/passthrough-triaged.txt` (a bug, with its task) or, when the
difference is intended, `tests/passthrough-accepted.txt` (with the reason).
A unit may not be in both, and a listed unit that passes through again fails
the run until its line is removed.

```sh
scripts/fetch-typescript-cases
cargo test --test corpus typescript_test_cases                     # 400 cases, fixed seed (PR CI)
TTC_TYPESCRIPT_CASES=all cargo test --release --test corpus typescript_test_cases   # every case (nightly)
```

When `package.json` moves to another TypeScript, update the manifest's
`typescript`, `commit` (the new package's `gitHead`), and tree ids in the
same change; the test fails while they disagree.

Contract 2 is checked over the same cases (TASK-717). For each case, a
second test in `tests/corpus.rs` writes the units twice, with a
`tsconfig.json` built from the case's `// @option` directives (typed by the
pinned TypeScript's own command-line parser; `noEmit` is always on): as they
are, and with every `.ts`/`.tsx` unit that no other unit imports renamed to
`.tt`/`.ttx` (a `.tt` module is imported by its extension, `docs/ai/tt.md`).
`tests/typescript-diagnostics.mjs` asks the pinned TypeScript, through its
API, for the diagnostics `tsc --noEmit -p` reports for the TypeScript
project, in `tsc`'s order of stages and with each range; the tt project is
asked through `ttc --server`'s `typedCheck` (with `includeTypes`) and
`ttc --check-types`, which must agree with each other. The two lists must be
equal as (file, range, code, message). A case is skipped, with the reason
counted in the output, when the harness lays out a file system the case
directory cannot (`@currentDirectory`, `@link`, `@symlink`, its own
`tsconfig.json`), when every configuration sets an option the pinned
TypeScript removed, when TypeScript reports a syntax or grammar error, or
when a renamed unit does not pass through. A difference is listed in
`tests/typed-parity-differences.txt` (the case, a tab, the difference's
signature as the failure prints it, a tab, `by-design` with the document
under `docs/` that states it or `defect` with the TASK that records the
repro, a tab, and the reason); the test fails on an unlisted difference, on
a listed signature that changed, and on a listed case that now agrees. The
first 40 cases compared are also run through `tsc -p` itself, whose printed
diagnostics must be the oracle's.

```sh
cargo test --test corpus typescript_cases_type_check                     # 80 cases, fixed seed (PR CI)
TTC_TYPED_CASES=all cargo test --release --test corpus typescript_cases_type_check   # every case (nightly)
TTC_TYPED_FILTER=bluebirdStaticThis cargo test --test corpus typescript_cases_type_check   # named cases
TTC_TYPED_CASES=all TTC_TYPED_SHARD=0/4 cargo test --test corpus typescript_cases_type_check  # every fourth case
```

The editor is checked against TypeScript's fourslash tests (TASK-718). At
the pinned commit they are Go files, `tsc/internal/fourslash/tests/*_test.go`,
and the manifest pins that tree too. `tests/editor_cases.rs` reads each
test's `content` literal with fourslash's own rules (`// @Filename` units,
`/*marker*/`, `{| "name": ... |}` and `[|range|]` markup), and the calls the
test makes before its first edit: quick info and hover, completions, go to
definition, find all references, rename, signature help, and semantic
tokens, at the markers or ranges they name. Renamed as above, the test
becomes an editor case asked through `ttc --server` and, as its twin,
through `tsgo --lsp`, compared by `parity_view`. A differing question is
listed in `tests/fourslash-differences.txt` in the format of the matrix's
list (a by-design line cites a document under `docs/`); the test fails on
an unlisted difference and on a listed one that now agrees.

```sh
cargo test --test editor_cases typescript_fourslash                      # 60 tests, fixed seed (PR CI)
TT_FOURSLASH=all cargo test --release --test editor_cases typescript_fourslash   # every test (nightly)
TT_FOURSLASH_FILTER=pathCompletions TT_FOURSLASH_VERBOSE=1 cargo test --test editor_cases typescript_fourslash
```

### Fuzz findings and the mutation pass

A crash input is a regression test. The fuzz targets' bodies live in
`fuzz/src/lib.rs`, and `tests/fuzz_regressions.rs` replays every file under
`fuzz/regressions/<target>/` through them on the stable toolchain, the way
typescript-go replays `testdata/fuzz/FuzzParser/` on every `go test`. Save a
minimized crash input there with the fix that makes it pass. An input that
still crashes is listed in `fuzz/regressions/expected-failures.txt` with the
task that fixes it and the crash it produces; the test fails when a listed
input stops crashing or crashes differently, so the fix removes its line.

The same test types every `.tt` and `.ttx` unit of `tests/cases` and
`tests/fixtures` prefix by prefix and deletes each of its characters, then
runs each mutant through the `--check`, emission, emit-map, projection, and
text-only editor pipelines. A crash whose signature is not listed fails with
a minimized input and the file name to save it under.

```sh
cargo test --test fuzz_regressions                                   # a fixed sample of 1000 mutants
TT_MUTATIONS=all cargo test --release --test fuzz_regressions        # every mutant (about a minute)
TT_MUTATIONS=5000 TT_MUTATION_SEED=7 cargo test --test fuzz_regressions  # another sample
node scripts/fuzz-seed-corpus                                        # seed fuzz/corpus/compile_any_bytes
```

CI runs the sample in `cargo test`, and every mutant in the scheduled run's
`exhaustive` job. The `Soak` workflow fuzzes each target for two minutes a
night from the seeded corpus and the committed crash inputs.

### The real-world diagnostic delta

`scripts/diagnostic-delta` is TypeScript's error-deltas check
(microsoft/typescript-error-deltas) for tt. It builds `ttc` at the merge
base of `HEAD` and a base branch in a temporary worktree
(`.tt-dev/delta-base`, removed afterwards, as `scripts/bench-compare` does)
and at `HEAD`, and runs both with `--check`, `--check-types`, and
`--out-dir` over the same programs: each project under
`tests/fixtures/practical-diagnostics/`, the two `tests/fixtures/mixed-source-*`
projects, every tt example on the website (`website/src/content.json`), and
the project `create-tt` scaffolds. It prints a report of every difference.
It fails when a run crashes with an internal compiler error that the base
did not, and when any output changed but the change touches no baseline
(`tests/baselines/reference/`, `tests/fixtures/**/expected.*`): a
behaviour change is pinned by a case.

```sh
node scripts/diagnostic-delta                          # against origin/main
node scripts/diagnostic-delta --base release-1.2 --report delta.md
```

Pull requests run it as the `delta` job of `CI`, with the report in the job
summary and as the `diagnostic-delta` artifact.

### Incremental answers equal fresh ones

`tests/incremental.rs` holds the engine to TypeScript's incremental-parser
rule (`compareTrees` in `src/testRunner/unittests/incrementalParser.ts`): a
project edited one keystroke at a time answers exactly as a project opened
fresh on the final text. Each sample opens a case or fixture unit in an
engine `Workspace`, retypes, replaces, or pastes text as seeded, asks the
engine a question between some edits, and then compares the diagnostics,
emitted TypeScript and declarations, service diagnostics, semantic tokens,
and hover, definition, and completion at fixed points with a fresh
workspace's. A difference is an engine defect, reported with the edit script
and the command that reruns that one sample.

```sh
cargo test --test incremental                                        # 8 samples, fixed seed (PR CI)
TT_INCREMENTAL=all cargo test --release --test incremental          # every sample (nightly)
TT_INCREMENTAL=40 TT_INCREMENTAL_SEED=7 cargo test --test incremental  # another sample
TT_INCREMENTAL_ONLY='<sample>' cargo test --test incremental         # the sample a failure names
```

언어 표면(구문, 판별 규칙, 에러 메시지, CLI 동작)을 바꾸는 변경은 컴파일러에
내장되는 [`docs/ai/tt.md`](./docs/ai/tt.md)를 함께 갱신해야 합니다. 사용자가
처음 접하는 기능이면 영문·한글 README에도 반영하세요. 공개 Rust API를 바꾸면
rustdoc과 doctest도 갱신하세요. doctest는 `cargo test`에서 함께 실행됩니다.

### 요청할 때만 도는 두 단계

기본 실행에는 없습니다. 각각 몇 분이 걸리고, 둘 다 "이 변경이 옳은가"에 혼자
답하지는 못하기 때문입니다. 이름을 대면 돕니다.

```sh
./scripts/ci coverage   # 줄 커버리지가 기준선 아래로 떨어지면 실패
./scripts/ci bench      # 이 revision과 merge base를 한 기계에서 비교
```

- **커버리지 하한선**은 목표치가 아니라 "떨어뜨리지 않는다"는 규칙입니다.
  TypeScript 툴체인이 있어야 기준선과 같은 숫자가 나옵니다(없으면 6포인트
  낮습니다). 기준선과 취약 목록은
  [`docs/tasks/TASK-224`](./docs/tasks/TASK-224-coverage-gate.md).
- **성능 비교**는 두 revision을 **한 기계에서** 재서 비율만 읽습니다. 다른
  기계에 기록된 기준선은 아무 뜻이 없기 때문입니다. 숫자만 보려면
  `cargo bench`. 임계값의 근거는
  [`docs/tasks/TASK-225`](./docs/tasks/TASK-225-performance-benchmarks.md).

`CI` 워크플로에도 같은 두 잡이 있고, 위의 트리거대로 자동으로 함께 돕니다.

### 버그를 찾으러 가는 것 — `Soak`

[`Soak`](./.github/workflows/soak.yml) 워크플로는 전체 코퍼스 차등 테스트와
퍼저 두 개를 돌립니다. 알려진 답을 확인하는 게 아니라 **모르는 것을 찾는**
쪽이라 시간이 들고 수확은 예측할 수 없습니다 — 파서·스캐너·codegen처럼 tt가
무엇을 주장하는지를 바꾼 변경에 dispatch하세요. 로컬 실행 명령은 그 파일의
머리말에 있습니다. 무엇을 어떻게 찾는지는
[`docs/tasks/TASK-223`](./docs/tasks/TASK-223-corpus-and-fuzzing.md).

## 릴리스

개발자 릴리스 절차, 자동·수동 경계, Beta·RC·Stable·Patch의 처리 기준은 [`docs/releasing.ko.md`](./docs/releasing.ko.md)에 단일 가이드로 정리합니다.

```sh
bun add -d @openload28/tt-lang@next @openload28/unplugin-tt@next
bunx @openload28/create-tt@next my-app
```

For the temporary, manual-only policy that points npm `latest` at a published
Nightly, see [Manual Nightly promotion](./docs/manual-nightly-latest.md).
