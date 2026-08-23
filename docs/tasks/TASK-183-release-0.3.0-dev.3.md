# TASK-183: release 0.3.0-dev.3

- **상태**: 진행 중
- **시작일**: 2026-08-23
- **완료일**: —
- **커밋**: —

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

없음.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

작업 중.
