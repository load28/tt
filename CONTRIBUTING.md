# 기여 가이드

## 개발 환경

저장소를 처음 열었거나 로컬 설치 상태가 확실하지 않으면 먼저 읽기 전용 진단을
실행합니다. 이 명령은 파일, 빌드 산출물, 에디터 설치를 변경하지 않습니다.

```sh
./scripts/doctor
```

- Rust — 버전은 `rust-toolchain.toml`이 고정하며 rustup이 자동으로 선택합니다
  (소비자에게 필요한 최소 버전은 `Cargo.toml`의 `rust-version`)
- 선택: Node.js + `typescript` (`npm i -g typescript`) — 없으면 tsc/node 통합
  테스트가 자동으로 skip됩니다. 온전한 검증을 위해 설치를 권장합니다.

```sh
cargo build
cargo test
```

## 로컬 개발 환경 (`scripts/setup`)

typed 경로(`--check-types`/`--types`/`--server`)까지 포함해 전부 로컬에서
돌려 보려면 setup 스크립트 하나면 됩니다. TypeScript 7 toolchain은 두 방식
중 하나로 연결합니다:

```sh
# A. 로컬 typescript-go 체크아웃을 쓸 때 (루트 경로 하나만 전달)
./scripts/setup --tsgo-root ~/dev/typescript-go

# B. TypeScript 7을 npm으로 쓸 때 (소비 프로젝트가 typescript@7을 설치)
./scripts/setup --tsgo-npm

# 이후에는 저장된 설정을 그대로 재사용
./scripts/setup
```

setup은 선택을 `.tt-dev/toolchain.json`에 저장하고(머신 로컬, 커밋 금지),
checkout 모드면 **현재 체크아웃된** typescript-go를 그 자리에서 빌드한 뒤
(git pull/checkout 같은 저장소 상태 변경은 절대 하지 않습니다 — 두 저장소의
갱신은 사용자가 직접), 현재 TT 체크아웃을 release 빌드하고, VSCode 확장을
빌드해 재설치(기존 설치는 삭제 후 설치)합니다.

테스트 프로젝트에서는 TT 전용 명령 없이 일반 패키지 매니저로 설치합니다:

```sh
pnpm add -D file:/path/to/tt/npm/tt-lang   # 재빌드 후에는 --force로 재설치
pnpm ttc --check-types src
```

launcher(`npm/tt-lang/bin/ttc.js`)가 저장소의 `target/release/ttc`를 실행하며,
checkout 모드의 `TTC_TSGO_*` 환경변수는 **그 child process에만** 주입됩니다 —
셸 프로파일은 건드리지 않습니다. VSCode 확장도 같은 `toolchain.json`을 읽어
CLI와 동일한 toolchain을 씁니다. 이 계층 전체는 임시 구조입니다: TT 패키지가
검증된 TypeScript 7을 직접 포함하게 되면 `scripts/setup`·`.tt-dev/`·
`npm/tt-lang/dev.js`를 함께 제거합니다 (`docs/tasks/TASK-090`).

## 절대 불변 원칙

어떤 변경도 이 두 계약을 깨뜨릴 수 없습니다 (자세한 내용은 [`AGENTS.md`](./AGENTS.md)):

1. 모든 유효한 TypeScript 파일은 그대로 유효한 `.tt` 파일이다 (바이트 단위 통과).
2. tt 수준 에러는 ttc가 직접 보고하고, 방출 코드는 타입 트릭 없는 순수
   TypeScript다 — ttc가 방출한 코드가 tsc 에러를 만들면 안 된다.

## 작업 절차 (필수)

모든 작업은 태스크 문서로 관리됩니다:

1. `docs/tasks/INDEX.md`에서 다음 번호를 확인하고 `docs/tasks/TEMPLATE.md`로
   태스크 문서를 만든 뒤 INDEX에 등록합니다.
2. 작업 중 결정·문제·범위 변경을 태스크 문서에 기록합니다.
3. 완료 시 검증 결과를 기록하고 상태를 갱신합니다.
4. 커밋 메시지는 태스크 ID로 시작합니다: `TASK-012: ...`.

## 머지 전 검증 게이트

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI가 동일한 게이트를 강제합니다. 새 기능에는 반드시 테스트를 추가하세요:

- 출력 형태 → `tests/compile.rs`
- TS 통과 계약 → `tests/passthrough.rs`
- 타입/런타임 의미 → `tests/integration.rs`
- **산출물 전체**가 계약인 것(방출된 TypeScript, 렌더된 진단) →
  `tests/fixtures/` 스냅샷. 픽스처 디렉터리에 `input.tt`를 넣고
  `UPDATE_EXPECT=1 cargo test --test snapshot`으로 기대 파일을 만든 뒤 **그 diff를
  읽으세요** — 그 diff가 리뷰 대상입니다. 부분 문자열 어서션은 여분의 문장이나
  어긋난 들여쓰기를 잡지 못합니다.

언어 표면(구문, 판별 규칙, 에러 메시지, CLI 동작)을 바꾸는 변경은 컴파일러에
내장되는 [`docs/ai/tt.md`](./docs/ai/tt.md)를 함께 갱신해야 합니다. 사용자가
처음 접하는 기능이면 영문·한글 README에도 반영하세요. 공개 Rust API를 바꾸면
rustdoc과 doctest도 갱신하세요. doctest는 `cargo test`에서 함께 실행됩니다.

### CI가 추가로 재는 것

로컬 게이트는 위 셋으로 충분합니다. CI는 여기에 두 개의 자를 더 댑니다.

- **커버리지 하한선** (`coverage` 잡). 줄 커버리지가 기준선 아래로 떨어지면
  실패합니다. 목표치가 아니라 "떨어뜨리지 않는다"는 규칙입니다. 로컬에서
  재보려면 `cargo llvm-cov --workspace --summary-only` — 단, TypeScript
  툴체인이 붙어 있어야 CI와 같은 숫자가 나옵니다(없으면 6포인트 낮습니다).
  기준선과 취약 목록은 [`docs/tasks/TASK-224`](./docs/tasks/TASK-224-coverage-gate.md).
- **성능 회귀** (`performance` 잡). `./scripts/bench-compare`가 이 revision과
  merge base를 **한 기계에서** 재서 비교합니다. 로컬에서는 `cargo bench`로
  숫자만 볼 수 있습니다. 임계값의 근거는
  [`docs/tasks/TASK-225`](./docs/tasks/TASK-225-performance-benchmarks.md).

## 개발 버전 배포

`Cargo.toml` 버전을 `X.Y.Z-dev.N` 형식으로 올려 `main`에 push하고 `CI`가
성공하면 [`Dev Release`](./.github/workflows/dev-release.yml)가 자동으로 개발
빌드를 배포합니다. `N`이 같은 일반 코드 push는 배포하지 않습니다. 수동 실행도
현재 Cargo 버전이 `X.Y.Z-dev.N`일 때만 허용합니다.

npm 버전은 `<개발버전>.<UTC 날짜>.<UTC 시간>.<run>.<attempt>`이며 모두 `dev`
dist-tag로 격리됩니다. 확장은 `0.<YYMMDD>.<HHMMSS>` 버전의 VSIX로 패키징해
같은 실행의 GitHub pre-release에 첨부합니다. GitHub Releases에서 VSIX를
내려받아 VS Code의 `Extensions: Install from VSIX...`로 설치합니다.

`Cargo.toml` 버전을 선행 식별자 없는 `X.Y.Z`로 올리면 `CI` 성공 뒤
[`Release`](./.github/workflows/release.yml)가 production npm 패키지와 GitHub
Release를 배포합니다. 저장소에는 대상 패키지 게시 권한의 `NPM_TOKEN` secret이
필요합니다.

```sh
bun add -d @load28/tt-lang@dev @load28/unplugin-tt@dev
bunx @load28/create-tt@dev my-app
```
