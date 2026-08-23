# TASK-181: release 0.3.0-dev.2

- **상태**: 취소
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: 44006eb

## 목적

Windows npm 패키지 이름 변경과 CI 차단 수정이 포함된 `0.3.0-dev.2` 개발
릴리스를 자동 배포한다.

## 범위

- 포함: `Cargo.toml`과 `Cargo.lock` 버전을 `0.3.0-dev.2`로 변경, main CI와
  자동 Dev Release 및 npm 게시 결과 확인
- 제외: production 배포, npm `latest` dist-tag 변경

## 의사결정

### 결정 1: 실패한 dev.1 대신 dev.2를 발행

- **상황**: dev.1 commit의 CI는 TASK-180 수정 전 코드를 검사하므로 자동 배포에
  도달할 수 없다. 같은 버전은 새 commit에서 배포 트리거가 되지 않는다.
- **검토한 대안**: dev.1 workflow를 수동 재실행하면 수정 전 SHA를 다시 사용한다.
  현재 commit에서 수동 실행하면 같은 기준 버전을 쓸 수 있지만 dev 버전 증가 기반
  자동 트리거를 검증하지 못한다.
- **선택과 근거**: 기준 버전을 dev.2로 올려 수정된 SHA의 CI 성공이 자동 Dev
  Release를 생성하게 한다.

## 작업 내역

- 2026-08-23: TASK-181을 등록하고 Cargo 기준 버전을 `0.3.0-dev.2`로 변경했다.
- 2026-08-23: dev.1에서 dev.2로의 channel 판정이 `development`임을 확인하고
  Node 릴리스 도구 테스트 10건과 세 검증 게이트를 통과했다.

## 이슈 및 해결

### 이슈 1: 변경한 Windows 패키지도 npm 스팸 탐지에 차단됨

- **증상**: Dev Release run `32641598660`에서 네 Unix 패키지는 게시됐지만
  `tt-lang-win32-x64-msvc` 게시가 npm 403 `Package name triggered spam
  detection`으로 실패했다. 사용자용 패키지와 VSIX 게시 작업은 실행되지 않았다.
- **원인**: npm에서 미사용인 이름이라는 사실만으로 신규 언스코프 패키지의 스팸
  탐지 통과 여부를 보장할 수 없다. 널리 쓰이는 target triplet 형식도 이 계정의
  신규 언스코프 이름에는 허용되지 않았다.
- **해결**: 이 릴리스는 부분 게시 상태이므로 취소하고, TASK-182에서 더 고유한
  Windows 패키지 이름을 적용한 뒤 새 개발 버전으로 다시 배포한다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

CI run `32641417739`는 통과했지만 Dev Release run `32641598660`이 Windows
패키지 게시에서 실패했다. 같은 immutable 버전의 Unix 패키지 네 개가 이미
게시됐으므로 dev.2를 재사용하지 않고 후속 개발 버전으로 대체한다.
