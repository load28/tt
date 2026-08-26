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
./scripts/ci
```

GitHub Actions의 [`CI`](./.github/workflows/ci.yml)는 `main`과 `release-X.Y`의
push·PR에서 자동으로 돕니다. `scripts/ci`는 같은 핵심 게이트를 로컬에서 재현하며,
PR을 열기 전에 먼저 실행해야 합니다.

| 단계 | 내용 |
| --- | --- |
| `agents` | 에이전트 진입점 계약(`CLAUDE.md`, `scripts/doctor`) |
| `rust` | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` |
| `npm` | npm 릴리스 도구와 프로젝트 초기화기 테스트 |
| `native` | TypeScript 7을 실제로 구동하는 타입 검사 모드 |
| `extension` | VS Code 확장 빌드와 서버 테스트 |

단계를 골라 돌릴 수 있습니다.

```sh
./scripts/ci rust              # Rust 게이트만
./scripts/ci --skip extension  # 확장 테스트만 빼고
./scripts/ci --list            # 단계 이름
```

`tsc`, rolldown, TypeScript 7 툴체인이 없으면 관련 테스트는 **실패가 아니라
조용히 스킵됩니다.** `scripts/ci`는 시작할 때 무엇이 없어서 어떤 검증이 사라지는지
경고하고 요약에서 한 번 더 알립니다 — 경고가 붙은 통과는 CI와 같은 통과가
아닙니다. `native` 단계만은 경고가 아니라 실패입니다. 그 단계의 존재 이유가
TypeScript 7 경로를 실제로 도는 것이기 때문입니다.

호스팅된 실행을 다시 확인해야 하면 Actions 탭에서 `CI` → **Run workflow**로도
시작할 수 있습니다.
새 기능에는 반드시 테스트를 추가하세요:

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

`CI` 워크플로에도 같은 두 잡이 있고, 워크플로 자체가 수동이므로 dispatch하면
함께 돕니다.

### 버그를 찾으러 가는 것 — `Soak`

[`Soak`](./.github/workflows/soak.yml) 워크플로는 전체 코퍼스 차등 테스트와
퍼저 두 개를 돌립니다. 알려진 답을 확인하는 게 아니라 **모르는 것을 찾는**
쪽이라 시간이 들고 수확은 예측할 수 없습니다 — 파서·스캐너·codegen처럼 tt가
무엇을 주장하는지를 바꾼 변경에 dispatch하세요. 로컬 실행 명령은 그 파일의
머리말에 있습니다. 무엇을 어떻게 찾는지는
[`docs/tasks/TASK-223`](./docs/tasks/TASK-223-corpus-and-fuzzing.md).

## 릴리스 모델

tt은 Microsoft TypeScript와 같이 `main`을 Nightly와 일반 개발의 기준으로 사용합니다.
작업 브랜치의 PR은 `main`에 squash merge합니다. Beta를 만들 때 장기 브랜치
`release-X.Y`를 한 번 만들고, 같은 브랜치를 RC·Stable·Patch까지 유지합니다.

- Nightly: 예약된 `main` CI 산출물, `X.Y.Z-dev.YYYYMMDD`, npm `next` 자동 게시
- Beta: 최신 `main`에서 만든 `release-X.Y`의 `X.Y.0-beta`, npm `beta`
- RC: Beta 브랜치를 `main`과 동기화한 뒤 `X.Y.1-rc`, npm `rc`
- Stable/Patch: 같은 릴리스 브랜치의 `X.Y.2`, `X.Y.3`…, npm `latest`

이 모델 도입 전에 `0.3.0`으로 Stable이 게시된 `release-0.3`은 기존 번호를 보존하고
`0.3.1`부터 Patch를 이어갑니다. 새 릴리스 라인부터 위 TypeScript 순서를 사용합니다.

TypeScript처럼 [`New Release Branch`](./.github/workflows/new-release-branch.yml),
[`Sync Release Branch`](./.github/workflows/sync-release-branch.yml),
[`Bump Release Version`](./.github/workflows/bump-release-version.yml)을 독립적으로
실행합니다. 세 액션은 각각 Beta 브랜치 생성, `main` 병합, 다음 단계 버전 증가만
담당합니다. 전용 `tt-release-automation` GitHub App 설치 토큰으로 push하므로 그
push가 CI를 자동으로 시작합니다. CI는 모든 플랫폼 바이너리와 VSIX를 만듭니다.
[`Publish Release`](./.github/workflows/release-publish.yml)는 성공한 CI 산출물만 게시하며
다시 빌드하지 않습니다. Nightly는 예약 CI 뒤 자동 게시합니다. Beta·RC·Stable·Patch는
성공한 릴리스 브랜치 CI가 `production` Environment에서 대기하며, 승인자가
`Approve and deploy`하면 게시됩니다. run ID와 npm tag는 자동으로 선택됩니다.
저장소의 `production` Environment는 `load28`을 필수 승인자로 지정합니다.

이 동작에는 저장소에 설치한 `tt-release-automation` GitHub App이 필요합니다. App의
저장소 `Contents` 권한은 `Read and write`로 제한합니다. Actions Variable
`RELEASE_APP_ID`에는 App ID를, Actions Secret `RELEASE_APP_PRIVATE_KEY`에는 생성한
private key PEM 전체를 등록합니다. Azure Key Vault 대신 GitHub Secret에 키를
보관하는 것만 TypeScript 환경과 다릅니다.

```sh
bun add -d @load28/tt-lang@next @load28/unplugin-tt@next
bunx @load28/create-tt@next my-app
```
