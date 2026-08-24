# TASK-201: 로컬 재설치와 tour 패키지 갱신

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: 이 커밋

## 목적

원격 `main`의 최신 코드를 로컬 개발 환경에 다시 설치하고, 두 tour 프로젝트가
현재 스코프 패키지를 사용하도록 의존성과 lockfile을 갱신한다.

## 범위

- 포함: `git pull --ff-only`, `scripts/setup` 재실행, `rl-tour`와 `rlx-tour`의 로컬 TT 패키지 갱신, 각 프로젝트 검증
- 제외: 언어·컴파일러 기능 변경, 패키지 버전 변경, 원격 배포

## 의사결정

### 결정 1: 로컬 패키지 경로는 유지하고 패키지명만 현재 스코프로 맞춘다

- **상황**: 두 tour 프로젝트는 저장소의 로컬 패키지를 직접 참조하지만 의존성 키는 개명 전 이름을 사용하고 있다.
- **검토한 대안**: 기존 별칭을 유지하면 변경은 적지만 현재 공개 패키지명과 예제가 어긋난다. 레지스트리 버전으로 바꾸면 로컬 최신 빌드를 검증할 수 없다.
- **선택과 근거**: `file:../tt/...` 경로는 유지하고 키를 `@load28/tt-lang`, `@load28/unplugin-tt`로 바꾼다. 각 package.json의 실제 `name`과 일치하며 로컬 최신 빌드를 그대로 소비한다.

### 결정 2: TypeScript npm 패키지 대신 typescript-go checkout을 사용한다

- **상황**: 초기 갱신에서 npm toolchain 설정에 맞춰 `typescript@7`을 추가했지만, 사용자는 로컬 typescript-go 빌드를 환경변수로 참조하도록 지정했다.
- **검토한 대안**: 각 tour에 `typescript@7`을 설치하면 프로젝트별로 독립적이지만 소스 checkout을 검증하지 않는다. `../typescript-go`를 setup에 연결하면 빌드 시간이 들지만 두 tour가 같은 로컬 tsgo 바이너리와 API를 사용한다.
- **선택과 근거**: 두 tour에서 `typescript` 의존성을 제거하고 `./scripts/setup --tsgo-root ../typescript-go`를 실행한다. 로컬 `@load28/tt-lang` 실행기가 저장된 checkout 경로를 `TTC_TSGO_ROOT`로 자식 컴파일러에 주입한다.

## 작업 내역

- 2026-08-24: `git pull --ff-only`로 `main`을 `5cec67c`에서 `f84223c`까지 fast-forward했다.
- 2026-08-24: `rl-tour`와 `rlx-tour`의 package.json, lockfile, 작업 트리 여부를 확인했다. 두 디렉터리는 독립 Git 저장소가 아니다.
- 2026-08-24: 두 tour의 의존성 키와 Vite import를 현재 `@load28` 스코프 이름으로 갱신했다.
- 2026-08-24: `./scripts/setup --tsgo-root ../typescript-go`로 tsgo native compiler와 API client, `ttc` release, VS Code 확장을 빌드하고 확장을 재설치했다.
- 2026-08-24: `.tt-dev/toolchain.json`이 checkout `/Users/seominyong/Downloads/source/typescript-go`를 가리키는지 확인했다.
- 2026-08-24: 두 tour에서 npm TypeScript 의존성을 제거한 뒤 `npm install`과 `bun install`로 lockfile과 로컬 설치를 갱신했다.
- 2026-08-24: `rl-tour` 정상 소스의 직접 `ttc --check`, 두 tour의 Vitest와 Vite production build, `rlx-tour`의 `ttc --check-types`를 실행했다.

## 이슈 및 해결

### 이슈 1: 초기 toolchain 선택을 사용자 요구에 맞게 정정

- **증상**: 저장된 npm toolchain 설정을 재사용해 두 tour에 `typescript@7` 의존성을 추가했다.
- **원인**: 최초 요청에서 toolchain 방식을 명시하지 않아 기존 `.tt-dev/toolchain.json`의 npm 설정을 따랐다.
- **해결**: 사용자 지시에 따라 npm TypeScript 의존성을 제거하고 `../typescript-go` checkout 빌드 방식으로 전환한다.

### 이슈 2: rl-tour 전체 검사에는 의도적 오류 데모가 포함된다

- **증상**: `npm run check`와 `npm run typecheck`가 `src/_errors-demo.tt`의 의도된 진단으로 종료 코드 1을 반환했다.
- **원인**: 두 명령이 `src` 전체를 대상으로 하며, 해당 파일은 컴파일러 오류 예제를 모은 고의 실패 입력이다. `--check-types`는 프로젝트 그래프 전체를 검사하므로 파일 인자를 좁혀도 tsconfig에 포함된 오류 데모를 읽는다.
- **해결**: 오류 데모를 제외한 정상 소스 전체를 `ttc --check`로 확인하고, 실제 import graph는 19개 테스트와 production build로 검증했다. 오류 데모 자체는 예상한 진단을 반환하는 것도 확인했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `rl-tour` 패키지 설치, 정상 소스 검사, 19개 테스트, production build
- [x] `rlx-tour` 패키지 설치, 구문·타입 검사, 3개 테스트, production build

## 결과

`main` 최신 소스를 typescript-go checkout 기반 로컬 환경으로 재설치했다. 두 tour는
현재 스코프 패키지와 Vite import를 사용하며 npm TypeScript 패키지 없이 로컬 tsgo를
참조한다. 변경 파일은 이 태스크 문서와 INDEX, 두 tour의 package.json·lockfile·
vite.config.ts, 그리고 `rl-tour/README.md`다.
