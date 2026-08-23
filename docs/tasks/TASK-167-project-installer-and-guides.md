# TASK-167: 프로젝트 설치 CLI와 통합 가이드

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: —

## 목적

처음 사용하는 개발자가 RL 저장소나 typescript-go 빌드 구조를 알지 않아도 새
TypeScript 프로젝트를 만들거나 기존 프로젝트에 RL을 추가할 수 있게 한다. 같은
설정을 자동 설치 CLI와 수동 문서, 공식 웹사이트에서 일관되게 제공한다.

## 범위

- 포함: 새 프로젝트 생성과 기존 프로젝트 초기화를 담당하는 npm CLI, package.json
  기반 프로젝트·패키지 매니저·번들러 감지, 안전한 설정 생성, 번들러별 수동 설치
  문서, 영문·한글 README와 공식 웹사이트 설치 가이드, 설치기 테스트.
- 제외: 임의 형태의 기존 번들러 설정을 문자열 휴리스틱으로 재작성하는 기능,
  TypeScript 또는 번들러 자체의 설치 마법사 대체, npm 배포와 버전 변경.

## 의사결정

### 결정 1: 생성과 마이그레이션을 `create-rl` npm 초기화 패키지가 소유한다

- **상황**: Rust 컴파일러 CLI는 소스 변환을 소유하지만 패키지 매니저 실행과
  번들러 설정은 JavaScript 프로젝트의 책임이다.
- **검토한 대안**: `rlc init`에 프로젝트 수정을 넣으면 컴파일러와 npm 생태계
  책임이 섞인다. 저장소의 `scripts/setup`을 확장하면 RL 기여자용 도구와 소비자
  설치가 결합된다. 독립 초기화 패키지는 `bun create` 표준 흐름에 들어간다.
- **선택과 근거**: `packages/create-rl`을 추가했다. 새 프로젝트 생성과 기존
  `package.json` 구조 변경은 이 패키지만 담당하고 컴파일러는 그대로 유지한다.

### 결정 2: 새 프로젝트의 패키지 매니저와 번들러는 Bun과 Vite로 고정한다

- **상황**: 새 프로젝트는 즉시 실행 가능한 하나의 기준이 필요하고 사용자는 생성
  패키지 매니저로 Bun을 지정했다.
- **검토한 대안**: 대화형 선택지는 자동화와 재현성을 낮춘다. 모든 조합의 템플릿은
  유지 범위를 불필요하게 늘린다. Bun+Vite 한 경로는 직접 `.rl` import를 실제
  애플리케이션으로 검증할 수 있다.
- **선택과 근거**: 새 프로젝트는 Bun으로 설치하고 Vite 설정, `main.ts` 진입점,
  `app.rl` 예제와 check/build 스크립트를 생성한다. 다른 패키지 매니저나 번들러
  플래그은 새 프로젝트에서 오류로 보고한다.

### 결정 3: 기존 번들러 설정은 재작성하지 않고 합성 래퍼를 생성한다

- **상황**: TypeScript/JavaScript 설정 파일은 함수, Promise, 배열, 플러그인으로
  임의 코드를 포함할 수 있어 안전한 일반 재작성 규칙이 없다.
- **검토한 대안**: 문자열 삽입은 프로젝트 계약의 휴리스틱 금지 원칙을 깨고 사용자
  코드를 훼손할 수 있다. 아무것도 자동화하지 않으면 마이그레이션 요구를 충족하지
  못한다. 기존 설정을 import하는 래퍼는 원본을 보존하면서 구조적으로 합성한다.
- **선택과 근거**: Vite/Rollup/Rolldown/webpack/Rspack/Farm에는
  `rl.*.config.mjs`와 전용 스크립트를 생성한다. 표준 설정 모듈이 없는 esbuild는
  패키지만 추가하고 정확한 플러그인 import를 출력한다. 여섯 래퍼와 esbuild 경계를
  테스트로 고정했다.

### 결정 4: 로컬 빌드는 `file:`이 아니라 npm 호환 로컬 레지스트리로 검증한다

- **상황**: 공개 전 패키지도 실제 설치와 같은 해석·패키징·플랫폼 바이너리 선택을
  거쳐야 한다. 로컬 경로 의존성은 이 과정을 우회한다.
- **검토한 대안**: `file:`/`link:`는 레지스트리 메타데이터와 optional platform
  package 설치를 검증하지 않는다. tarball 직접 설치도 create 패키지가 `latest`
  의존성을 해석하는 경로와 다르다. Verdaccio는 npm 프로토콜과 upstream proxy를
  제공한다.
- **선택과 근거**: `scripts/verdaccio.local.yaml`과
  `scripts/publish-local-registry.mjs`를 추가했다. 게시기는 현재 플랫폼의 release
  바이너리와 rl-lang/unplugin-rl/create-rl을 고유 prerelease로 게시한다. 설치기의
  `--registry`는 Bun 설치와 생성 프로젝트 `bunfig.toml`에 같은 URL을 연결한다.

## 작업 내역

- 2026-08-23: 기존 rl-lang 패키지, unplugin 어댑터, 개발자용 setup, README와
  웹사이트 구조를 조사하고 TASK-167을 등록했다.
- 2026-08-23: `packages/create-rl`에 Bun+Vite 프로젝트 생성, 기존 프로젝트
  초기화, 패키지 매니저·번들러 감지, 설정 합성 래퍼, `--registry`를 구현했다.
- 2026-08-23: 생성·레지스트리 설정·기존 Vite 보존·전체 선언형 번들러 래퍼·
  esbuild 수동 경계·Bun/Vite 기준을 검증하는 7개 테스트를 추가하고 CI에 연결했다.
- 2026-08-23: 로컬 Verdaccio 설정과 현재 플랫폼용 패키지 게시 스크립트를 추가했다.
  darwin-arm64 release 바이너리 및 네 패키지를 게시하고 레지스트리에서만 생성한
  프로젝트의 타입 검사와 Vite 빌드를 통과시켰다.
- 2026-08-23: 영문·한글 설치 가이드와 README, npm README, AI 가이드를 갱신했다.
  자동 생성, 기존 프로젝트, 컴파일러 수동 설치, 일곱 번들러 연결, 점진적 파일
  마이그레이션, 로컬 레지스트리 절차를 기록했다.
- 2026-08-23: 웹사이트에 `/install`, `/ko/install`을 추가하고 홈페이지 설치
  명령을 Bun 기준으로 바꿨다. typecheck와 33개 페이지 prerender를 통과시켰다.
- 2026-08-23: 릴리스 워크플로에 create-rl 게시와 독립 버전 unplugin-rl의 조건부
  게시를 연결하고 stamp-version이 create-rl도 Cargo 버전으로 스탬프하게 했다.

## 이슈 및 해결

### 이슈 1: 공개 npm 패키지가 아직 존재하지 않음

- **증상**: 생성 프로젝트에서 `bun install`을 실행하면 `rl-lang`과
  `unplugin-rl`에 대해 registry.npmjs.org 404가 발생했다.
- **원인**: 저장소에는 패키징 코드가 있지만 해당 패키지는 공개 레지스트리에 아직
  게시되지 않았다. 기존 릴리스 워크플로도 unplugin-rl과 create-rl을 게시하지 않았다.
- **해결**: 릴리스 워크플로에 두 패키지를 연결했다. 현재 작업 검증은 실제 로컬
  Verdaccio에 모든 패키지를 게시한 뒤 동일 npm 프로토콜로 수행했다.

### 이슈 2: HTML이 `.rl`을 직접 진입점으로 가리키면 Vite root URL이 절대 경로가 됨

- **증상**: 첫 생성 템플릿의 Vite 빌드가 `no such file or directory:
  /src/main.rl`로 실패했다.
- **원인**: HTML의 `/src/main.rl`은 Vite root-relative URL이지만 unplugin의 공통
  resolver에는 파일시스템 절대 경로처럼 전달된다.
- **해결**: Vite가 소유하는 표준 `src/main.ts` 진입점이 상대 지정자
  `./app.rl`을 import하게 했다. 이는 기존 TypeScript 프로젝트의 점진적
  마이그레이션과 같은 모듈 경계이며 최종 빌드로 검증했다.

### 이슈 3: 웹사이트 prerender가 샌드박스 포트 제한으로 실패

- **증상**: 최초 `bun run build`가 `listen EPERM: operation not permitted ::1`로
  실패했다.
- **원인**: TanStack Start prerender가 로컬 preview server를 열지만 샌드박스가
  listen을 차단했다.
- **해결**: 동일 빌드를 승인된 외부 실행으로 다시 수행해 33개 페이지를 모두
  prerender했다.

## 검증

- [x] `bun test packages/create-rl/test/installer.test.mjs` — 7 passed
- [x] 로컬 레지스트리 E2E — publish → create → install → check → Vite build
- [x] 웹사이트 `bun run typecheck`와 `bun run build` — 33 pages
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

새 프로젝트와 기존 프로젝트 모두 하나의 create-rl 명령으로 설정할 수 있다.
공개 릴리스 전에는 로컬 npm 호환 레지스트리에서 동일 설치 경로를 사용할 수 있고,
자동·수동 절차가 README, 상세 가이드, AI 가이드, 공식 웹사이트에 동기화됐다.
