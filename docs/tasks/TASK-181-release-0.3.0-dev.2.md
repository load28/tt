# TASK-181: release 0.3.0-dev.2

- **상태**: 진행 중
- **시작일**: 2026-08-23
- **완료일**: —
- **커밋**: —

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

없음.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

작업 중.
