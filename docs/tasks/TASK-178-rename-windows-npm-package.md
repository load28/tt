# TASK-178: Windows npm 플랫폼 패키지 이름 변경

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: —

## 목적

npm 스팸 탐지에 차단된 `tt-lang-win32-x64` 패키지 이름을 배포 가능한 이름으로
교체한다. 런타임 플랫폼 식별자와 npm 패키지 이름을 분리해 Windows에서도 올바른
바이너리 패키지를 찾게 한다.

## 범위

- 포함: Windows npm 패키지를 `tt-lang-win32-x64-msvc`로 변경, 생성·설치·탐색·
  로컬 레지스트리·문서 참조 갱신, 회귀 테스트
- 제외: Node.js의 `process.platform === "win32"` 식별자 및 Rust target 변경,
  이미 게시된 다른 플랫폼 개발 패키지 삭제

## 의사결정

### 결정 1: 플랫폼 키와 npm 패키지 이름을 명시적으로 매핑

- **상황**: Node.js와 빌드 target은 `win32-x64`를 사용하지만 npm에 게시할
  이름만 `windows-x64`로 바꿔야 한다.
- **검토한 대안**: 내부 키까지 `windows-x64`로 바꾸면 Node.js 플랫폼 값과
  변환 규칙이 여러 위치에 생긴다. 문자열 치환으로 Windows만 예외 처리하면
  패키지 생성기와 런처가 쉽게 어긋난다. 공용 manifest는 대상별 런타임 값과
  게시 이름의 차이를 하나의 계약으로 표현한다.
- **선택과 근거**: 플랫폼 manifest에 `key`, npm `package`, `os`, `cpu`를
  함께 기록하고 생성기·런처·로컬 게시기가 이를 소비한다.

### 결정 2: Windows 게시 이름은 `tt-lang-win32-x64-msvc`

- **상황**: 기존 이름을 대체할 수 있고 npm에서 사용되지 않는 이름을 선택해야
  한다.
- **검토한 대안**: `windows-x64`는 읽기 쉽지만 Node.js와 Rust 생태계의 target
  명칭과 다르다. `win32-x64-msvc`는 길지만 SWC, Rollup, Lightning CSS의
  Windows x64 MSVC 네이티브 패키지와 같은 규칙이다.
- **선택과 근거**: `tt-lang-win32-x64-msvc`를 선택했다. 2026-08-23에 npm
  registry의 `npm view`가 이 이름에 404를 반환해 미사용 상태를 확인했다.

## 작업 내역

- 2026-08-23: TASK-178을 등록하고 `win32-x64`의 생성·의존성·런타임 탐색·
  로컬 게시·문서 참조를 조사했다.
- 2026-08-23: SWC, Rollup, Biome, esbuild, Lightning CSS의 npm 패키지 이름을
  조회하고 네 후보 이름의 registry 사용 여부를 확인했다.
- 2026-08-23: `platforms.json`을 추가해 플랫폼 키, npm 패키지 이름, npm의
  `os`·`cpu` 조건을 한곳에서 관리하도록 했다. 패키지 생성기, `tt-lang` 런처,
  로컬 레지스트리 게시기가 이 manifest를 소비하도록 변경했다.
- 2026-08-23: dev·production 워크플로가 생성된 플랫폼 패키지 디렉터리를
  이름 가정 없이 게시하도록 변경하고 Windows release archive 경로도 생성기의
  반환값을 사용하도록 변경했다.
- 2026-08-23: Windows 매핑, optional dependency 일치, 생성된 package manifest를
  검증하는 Node 회귀 테스트 3건을 추가했다. YAML 파싱, Node 테스트 10건과 전체
  검증 게이트를 통과했다.

## 이슈 및 해결

### 이슈 1: 기존 Windows 패키지 이름의 npm 403

- **증상**: `tt-lang-win32-x64` 최초 게시가 npm의 `Package name triggered spam
  detection` 403으로 실패했다.
- **원인**: 같은 토큰으로 다른 플랫폼 패키지 네 개가 게시됐으므로 인증 문제가
  아니라 이름 단위 registry 차단이다.
- **해결**: npm에서 미사용임을 확인한 Rust 생태계 표준형 이름
  `tt-lang-win32-x64-msvc`로 교체했다. 실제 registry 수용 여부는 개발 릴리스로
  최종 확인한다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

Windows 빌드 키는 `win32-x64`로 유지하면서 게시·설치되는 npm 이름만
`tt-lang-win32-x64-msvc`로 변경했다. 모든 소비자가 공용 플랫폼 manifest를
사용하므로 생성 이름과 런타임 탐색 이름이 일치한다.
