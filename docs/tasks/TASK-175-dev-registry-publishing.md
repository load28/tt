# TASK-175: npm·VS Code 개발 버전 자동 배포

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: 3f9a899

## 목적

정식 릴리스 전에도 npm 레지스트리와 VS Code Marketplace에서 개발 빌드를
반복 설치해 검증할 수 있게 한다. `main`의 기준 버전이 올라가고 CI가 통과하면
고유한 날짜·시간 버전을 만들어 자동 배포한다.

## 범위

- 포함: CI 성공 뒤 기준 버전 상승 감지, UTC 날짜·시간 개발 버전 생성, npm
  `dev` dist-tag 배포, VS Code Marketplace pre-release 배포, 수동 재배포,
  버전 생성 테스트와 운영 문서
- 제외: 실제 레지스트리 계정·토큰 생성, 정식 `vX.Y.Z` 릴리스 흐름 변경,
  crates.io 배포

## 의사결정

### 결정 1: 배포 트리거는 성공한 `main` CI와 수동 실행

- **상황**: 버전 변경 직후 검증되지 않은 산출물이 레지스트리에 먼저 올라가지
  않으면서, 같은 기준 버전도 장애 복구나 반복 검증을 위해 다시 배포할 수 있어야
  한다.
- **검토한 대안**: `Cargo.toml` push를 직접 받으면 단순하지만 CI와 배포가
  경주한다. 태그 전용이면 개발 배포마다 Git 태그가 쌓인다. 성공한 CI의
  `workflow_run`을 받으면 검증을 선행하며 `workflow_dispatch`를 더해 재실행할 수
  있다.
- **선택과 근거**: 성공한 `main` push CI에서 이전 커밋보다 Cargo 기준 버전이
  상승한 경우 자동 배포한다. 수동 실행은 같은 기준 버전의 추가 배포에 사용한다.

### 결정 2: npm과 Marketplace 제약에 맞는 두 표현을 사용

- **상황**: npm은 prerelease 식별자를 지원하지만 VS Code Marketplace는 확장
  버전에 `major.minor.patch` 숫자 세 칸만 허용한다.
- **검토한 대안**: 두 레지스트리에 같은 SemVer를 쓰면 Marketplace 배포가
  불가능하다. VS Code에서 날짜를 빼면 잦은 개발 배포 버전을 수동 관리해야 한다.
- **선택과 근거**: npm은
  `<Cargo>-dev.<YYYYMMDD>.<HHMMSS>.<run>.<attempt>`, VS Code는
  `0.<YYMMDD>.<HHMMSS>`를 쓰고 `--pre-release`로 채널을 구분한다. UTC 시각과
  GitHub 실행 번호가 npm 버전 충돌을 막고, Marketplace의 숫자 세 칸 계약도
  지킨다.

### 결정 3: npm 개발 빌드는 `dev` dist-tag로 격리

- **상황**: 개발 배포가 기본 설치의 `latest`를 덮으면 안정 버전 사용자가
  의도하지 않은 개발 빌드를 받는다.
- **검토한 대안**: 기본 `latest`는 설치가 짧지만 안정 채널을 오염시킨다.
  별도 `dev` 태그는 사용자가 명시적으로 선택해야 한다.
- **선택과 근거**: 모든 npm 개발 패키지를 `--tag dev`로 게시한다. 설치자는
  `tt-lang@dev`, `create-tt@dev`, `unplugin-tt@dev`로 선택한다.

### 결정 4: 개발 설치기는 같은 채널의 도구 체인을 설치

- **상황**: `create-tt@dev`가 기존 상수 `latest`를 그대로 쓰면 생성된 프로젝트가
  개발 컴파일러와 번들러 플러그인을 검증하지 않는다.
- **검토한 대안**: CI에서 설치기 소스를 문자열 치환하면 배포 스크립트와 소스의
  계약이 갈린다. 설치기가 자신의 패키지 버전을 읽으면 모든 `-dev.*` 배포에 같은
  규칙이 적용된다.
- **선택과 근거**: 설치기 버전이 `-dev.`를 포함하면 생성 manifest의 `tt-lang`과
  `unplugin-tt`를 `dev`로 기록하고, 정식 및 저장소 플레이스홀더는 기존 `latest`를
  유지한다. 단위 테스트로 세 경우를 고정했다.

## 작업 내역

- 2026-08-23: 기존 정식 릴리스, npm 스탬프, 확장 패키징 구조와 공식 npm·VS
  Code 개발 배포 제약을 확인하고 태스크를 등록했다.
- 2026-08-23: `dev-versions.mjs`와 단위 테스트를 추가하고, 기존 스탬프 도구가
  unplugin과 VS Code manifest의 배포용 버전도 선택적으로 기록하게 확장했다.
- 2026-08-23: 성공한 `main` CI의 Cargo 버전 상승과 수동 실행을 받는
  `.github/workflows/dev-release.yml`을 추가했다. 다섯 플랫폼 바이너리 빌드,
  npm `dev` 게시, 확장 pre-release 게시와 VSIX 보관을 연결했다.
- 2026-08-23: 개발 `create-tt`가 같은 `dev` 채널 의존성을 생성하도록 바꾸고,
  정규 CI가 버전 도구 테스트도 실행하게 했다. `CONTRIBUTING.md`에 secret,
  버전 형식, 설치 명령을 기록했다.
- 2026-08-23: Node 단위 테스트 10개, 확장 grammar/TypeScript 빌드, YAML 파싱,
  실제 `vsce 3.9.2` pre-release VSIX 패키징과 전체 검증 게이트를 통과했다.

## 이슈 및 해결

### 이슈 1: sandbox에서 vsce 다운로드 DNS 해석 실패

- **증상**: 실제 패키징 검증에서 npm registry 요청이 `ENOTFOUND`로 실패했다.
- **원인**: 제한된 실행 환경의 네트워크 접근으로 `@vscode/vsce@3.9.2`를 처음
  내려받지 못했다.
- **해결**: 승인된 네트워크 실행으로 같은 고정 버전을 내려받아 패키징했다.
  생성된 VSIX의 `Microsoft.VisualStudio.Code.PreRelease=true`와 manifest 버전을
  직접 확인했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

개발 배포가 정식 릴리스와 분리된 채널로 자동화됐다. 변경 파일은 개발 릴리스
워크플로, npm 버전·스탬프 도구, 설치기 채널 판별과 테스트, CI 및 운영 문서다.
실제 원격 게시에는 저장소 secret 등록 뒤 `main`의 다음 기준 버전 상승 또는
수동 `Dev Release` 실행이 필요하다.
