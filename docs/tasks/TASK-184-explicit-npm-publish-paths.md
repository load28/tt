# TASK-184: npm publish 로컬 경로 명시

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: e8a9721

## 목적

npm이 사용자용 패키지 디렉터리를 GitHub 축약형으로 해석하지 않도록 dev와
production 배포 워크플로의 로컬 경로를 명시한다.

## 범위

- 포함: 세 사용자용 npm 패키지의 publish 경로 수정, 워크플로 회귀 테스트
- 제외: 패키지 이름과 배포 순서 변경

## 의사결정

### 결정 1: 모든 고정 로컬 경로에 `./` 접두사 사용

- **상황**: `npm publish npm/tt-lang`이 로컬 폴더가 아니라 GitHub 저장소
  `npm/tt-lang`으로 해석됐다. 같은 형태의 다른 두 패키지와 production
  워크플로에도 같은 잠재 오류가 있다.
- **검토한 대안**: 각 패키지 디렉터리로 `working-directory`를 지정할 수 있지만
  단계 수가 늘고 dev의 연속 게시가 분리된다. tarball을 먼저 만들면 경로가
  명확하지만 불필요한 패키징 단계가 추가된다. 상대 경로 앞에 `./`를 붙이면 npm
  package spec이 로컬 디렉터리로 명확해진다.
- **선택과 근거**: dev와 production 워크플로의 세 고정 경로를 모두 `./`로
  시작하게 하고 테스트로 고정한다.

## 작업 내역

- 2026-08-23: Dev Release run `32642253772`의 실패 로그에서 npm이
  `ssh://git@github.com/npm/tt-lang.git`을 조회한 사실을 확인했다.
- 2026-08-23: dev와 production 워크플로의 세 사용자용 패키지 경로에 `./`를
  붙이고, 두 워크플로를 검사하는 Node 회귀 테스트를 추가했다.
- 2026-08-23: Node 릴리스 도구 테스트 11건과 세 검증 게이트를 통과했다.

## 이슈 및 해결

없음.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

두 배포 워크플로가 `tt-lang`, `unplugin-tt`, `create-tt`를 모두 명시적인 로컬
디렉터리 package spec으로 게시한다. 회귀 테스트가 `./` 없는 고정 publish
경로의 재도입을 차단한다.
