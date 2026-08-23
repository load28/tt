# TASK-186: npm 패키지를 `@load28` 스코프로 통일

- **상태**: 진행 중
- **시작일**: 2026-08-23
- **완료일**: —
- **커밋**: —

## 목적

npm의 유사 이름 보호와 기존 소유권 문제를 피하고 패키지 소유권을 한곳에서
관리하도록 모든 npm 배포를 `@load28` 조직 스코프로 통일한다.

## 범위

- 포함: 메인·플랫폼·unplugin·설치기 패키지를 `@load28` 스코프로 변경,
  코드·테스트·문서·로컬 배포 참조 갱신, npmjs registry 고정
- 제외: VS Code extension publisher와 GitHub Release 이름 변경

## 의사결정

### 결정 1: 모든 npm 패키지를 `@load28` 조직 스코프에 배치

- **상황**: `unplugin-tt`은 npm이 `unplugin-dts`와 너무 유사하다고 차단했고,
  `create-tt`은 다른 사용자가 이미 소유한다.
- **검토한 대안**: 고유한 언스코프 이름은 개별 패키지마다 npm 보호 정책을 다시
  통과해야 한다. 사용자 계정 스코프는 즉시 쓸 수 있지만 프로젝트 소유권이 개인
  계정에 묶인다. 사용자가 보유한 `load28` 조직 스코프는 패키지군을 한 namespace로
  관리한다.
- **선택과 근거**: 기존 이름 앞에 `@load28/`을 붙여 `@load28/tt-lang`, 다섯
  `@load28/tt-lang-<platform>`, `@load28/unplugin-tt`, `@load28/create-tt`로
  통일한다. 격리한 npm 설정으로 npmjs.org를 조회해 대표 이름들이 모두 E404인
  미등록 상태를 확인했다.

## 작업 내역

- 2026-08-23: 기존 unplugin 패키지와 후보 이름을 registry에서 조회했다.
- 2026-08-23: `create-tt@0.0.1`이 별도 사용자 소유임을 확인하고 설치기 이름도
  같은 태스크에서 변경하기로 했다.
- 2026-08-23: 사용자가 `load28` 조직 스코프를 지정해 모든 npm 배포를 같은
  스코프로 통일하기로 범위를 확장했다.
- 2026-08-23: 로컬 `@load28` 설정이 GitHub Packages를 가리키는 것을 확인하고,
  격리 설정으로 npmjs.org에서 최종 패키지 이름의 미등록 상태를 재확인했다.
- 2026-08-23: 메인·플랫폼·unplugin·설치기 manifest와 런타임 탐색, 생성기,
  두 배포 워크플로, 설치기, 에디터 로컬 탐색, 문서를 `@load28` 이름으로 갱신했다.
- 2026-08-23: 프로젝트 `.npmrc`와 각 publish manifest에 npmjs.org를 명시하고
  모든 배포 패키지가 `@load28`에 속하는지 검사하는 회귀 테스트를 추가했다.
- 2026-08-23: Node 패키지·설치기 테스트 21건, 세 정적 패키지의
  `npm pack --dry-run`, 배포 YAML 파싱, 에디터 scoped 경로 테스트 7건을 통과했다.
- 2026-08-23: 세 검증 게이트를 통과했다.

## 이슈 및 해결

### 이슈 1: scoped import 정규식의 slash 이스케이프 누락

- **증상**: 설치기 테스트 파일이 `Invalid regular expression flags` 문법 오류로
  실행되지 않았다.
- **원인**: 패키지 이름 앞에 `@load28/`을 붙일 때 정규식 리터럴의 scope slash가
  이스케이프되지 않았다.
- **해결**: scope와 subpath의 slash를 모두 이스케이프하고 테스트를 재실행한다.

### 이슈 2: 사용자 npm 캐시의 소유권 오류

- **증상**: `npm pack --dry-run`이 사용자 캐시의 root 소유 파일 때문에 EPERM으로
  실패했다.
- **원인**: 이전 npm 실행이 사용자 캐시에 다른 소유권의 임시 파일을 남겼다.
- **해결**: 사용자 캐시를 수정하지 않고 이 작업 전용 `/tmp` 캐시로 패키징을
  검증한다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

메인 패키지, 다섯 플랫폼 패키지, unplugin, 설치기까지 npm에 게시되는 여덟
패키지가 모두 `@load28` 스코프를 사용한다. 런타임 탐색과 설치기가 scoped
package spec을 소비하며 배포와 로컬 개발은 npmjs.org를 명시적으로 사용한다.
