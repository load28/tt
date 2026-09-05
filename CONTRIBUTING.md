# 기여 가이드

## 개발 환경

먼저 개발 환경을 진단합니다.

```sh
./scripts/doctor
```

- Rust 버전: `rust-toolchain.toml`
- Node.js 의존성: `npm ci`
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

## 릴리스

개발자 릴리스 절차, 자동·수동 경계, Beta·RC·Stable·Patch의 처리 기준은 [`docs/releasing.ko.md`](./docs/releasing.ko.md)에 단일 가이드로 정리합니다.

```sh
bun add -d @openload28/tt-lang@next @openload28/unplugin-tt@next
bunx @openload28/create-tt@next my-app
```
