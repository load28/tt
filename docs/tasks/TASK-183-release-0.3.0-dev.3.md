# TASK-183: release 0.3.0-dev.3

- **상태**: 취소
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: d69d5a6

## 목적

고유한 Windows npm 패키지 이름이 포함된 `0.3.0-dev.3` 개발 릴리스를 자동
배포하고 npm 패키지와 GitHub VSIX pre-release를 확인한다.

## 범위

- 포함: Cargo 기준 버전을 `0.3.0-dev.3`으로 변경, main CI와 자동 Dev Release,
  npm `dev` 패키지와 VSIX 결과 확인
- 제외: production 배포, npm `latest` dist-tag 변경

## 의사결정

### 결정 1: 부분 게시된 dev.2 대신 dev.3 사용

- **상황**: dev.2의 Unix 플랫폼 패키지 네 개는 immutable registry 버전으로
  게시됐지만 Windows 패키지에서 작업이 실패했다. 같은 derived 버전은 다시
  게시할 수 없다.
- **검토한 대안**: 실패 작업만 재실행하면 이미 게시된 Unix 패키지에서 충돌한다.
  게시 루프를 재개형으로 바꾸는 방법은 배포 구조 변경이 필요하다. 새 기준 버전은
  현재 자동 버전 증가 계약을 그대로 사용한다.
- **선택과 근거**: Cargo 기준 버전을 `0.3.0-dev.3`으로 올려 새 immutable
  registry 버전으로 전체 자동 배포를 다시 실행한다.

## 작업 내역

- 2026-08-23: TASK-183을 등록하고 Cargo 기준 버전을 `0.3.0-dev.3`으로 변경했다.
- 2026-08-23: Node 릴리스 도구 테스트 10건과 세 검증 게이트를 통과했다.

## 이슈 및 해결

### 이슈 1: 사용자용 패키지 경로가 GitHub 축약형으로 해석됨

- **증상**: Dev Release run `32642253772`에서 모든 플랫폼 패키지는 게시됐지만
  `npm publish npm/tt-lang`이 `git ls-remote ssh://git@github.com/npm/tt-lang.git`
  실행 후 exit code 128로 실패했다.
- **원인**: `npm/tt-lang`은 `./` 없는 package spec이라 npm이 로컬 디렉터리가
  아니라 GitHub의 `npm/tt-lang` 저장소 축약형으로 해석했다.
- **해결**: 이 릴리스는 부분 게시 상태로 취소하고 TASK-184에서 모든 사용자용
  로컬 패키지 경로를 `./`로 명시한다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

CI run `32642090879`는 통과했고 Windows를 포함한 플랫폼 패키지 다섯 개도
성공적으로 게시됐다. 사용자용 패키지 게시가 로컬 경로 해석 오류로 중단됐으므로
dev.3은 후속 개발 버전으로 대체한다.
