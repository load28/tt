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
- 출력 형태 → `tests/compile.rs`
- TS 통과 계약 → `tests/passthrough.rs`
- 타입/런타임 의미 → `tests/integration.rs`
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
- `// @rewriteImports: js|ts|off` and `// @noVerify: true` are ttc's
  `--rewrite-imports` and `--no-verify`. There are no other options; an
  unknown directive fails the case.

Case names must be distinct across `tests/cases`, because each case writes
its baselines as `tests/baselines/reference/<name>.<kind>`:

| Baseline | Contents |
| --- | --- |
| `<name>.ts` | every unit, then every file `ttc --out-dir` wrote (support modules by name only) |
| `<name>.errors.txt` | what `ttc --out-dir` and `ttc --check-types` report, then what `tsc` reports on the emitted TypeScript; absent when all three succeed |
| `<name>.map.txt` | the source-to-output mappings of the editor projection (`ttc::emit_mapped`) |
| `<name>.types` | the engine's hover for each classified identifier, under its source line |

Create or refresh the baselines, then read the diff before committing it with
the change:

```sh
UPDATE_EXPECT=1 cargo test --test case_baselines
git diff -- tests/baselines
TT_CASES=<name fragment> cargo test --test case_baselines   # a few cases while iterating
```

The `.ts`, `.errors.txt`, and `.types` baselines need the pinned TypeScript
(`npm ci`). Without it they are skipped, `TTC_REQUIRE_TSGO=1` turns the skip
into a failure, and `UPDATE_EXPECT=1` refuses to run.

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

A TypeScript twin, the same name with `.ts` or `.tsx` and the same units
with `.ts`/`.tsx` for `.tt`/`.ttx`, is asked the same questions at the same
markers through `tsgo --lsp`. The per-file verbs are compared only when the
twin's text is the source's. Every answer that differs from TypeScript's is
shown in the case's baseline and listed in
`tests/baselines/reference/editor/failingParity.txt`, which is a baseline
too: a new difference and a fixed one both change it.

```sh
UPDATE_EXPECT=1 cargo test --test editor_cases
git diff -- tests/baselines/reference/editor
TT_CASES=<name fragment> cargo test --test editor_cases   # a few cases while iterating
```

### Managing the baselines

Every reference file is a baseline some test compares: everything under
`tests/baselines/reference/` (the editor cases own `editor/`), and the `expected.*` files under
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

```sh
node scripts/check-baselines --run            # the baseline suites, then the check
node scripts/check-baselines --run --accept   # regenerate, and delete unused baselines
git diff -- tests/baselines tests/fixtures    # review before committing
```

`./scripts/ci rust` runs `cargo test` with tracking and then the check. The
hosted `CI` does the same, then regenerates every baseline and fails when the
tree differs from the commit, listing missing, modified, and unused
baselines and uploading the difference as the `fix_baselines.patch`
artifact. `git apply fix_baselines.patch` reproduces it locally.

### TypeScript's own test cases

Contract 1 is checked against TypeScript's test suite. `tests/typescript-cases.json`
pins the microsoft/TypeScript commit the pinned `typescript` package was
built from (its `gitHead`) and the tree ids of `tests/cases/compiler` and
`tests/cases/conformance` there; `scripts/fetch-typescript-cases` makes a
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
