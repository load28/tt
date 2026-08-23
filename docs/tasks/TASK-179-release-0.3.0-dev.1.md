# TASK-179: release 0.3.0-dev.1

- **상태**: 취소
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: 81db313

## 목적

개발 버전 자동 배포의 첫 기준 버전인 `0.3.0-dev.1`을 선언한다. 변경된 Windows
npm 패키지 이름을 포함한 전체 개발 도구 체인을 npm과 GitHub pre-release에
배포한다.

## 범위

- 포함: `Cargo.toml`과 `Cargo.lock`의 기준 버전을 `0.3.0-dev.1`로 변경,
  검증 후 main push를 통한 자동 개발 배포
- 제외: production 배포, `latest` dist-tag 변경

## 의사결정

### 결정 1: 첫 자동 개발 기준 버전은 `0.3.0-dev.1`

- **상황**: 이름 변경 커밋은 Cargo 버전이 같아 자동 배포되지 않는다. npm의 새
  Windows 이름이 실제 게시되는지 확인하려면 명시적인 개발 버전 증가가 필요하다.
- **검토한 대안**: workflow 수동 실행은 현재 정식 버전 `0.3.0`에서 거부되며
  버전 기반 자동 배포 계약도 검증하지 못한다. `0.3.1`은 production 배포이므로
  개발 단계라는 현재 의도와 다르다.
- **선택과 근거**: `0.3.0-dev.1`로 올려 CI 성공 뒤 Dev Release가 자동으로
  실행되게 한다.

## 작업 내역

- 2026-08-23: TASK-179를 등록하고 `Cargo.toml` 및 `Cargo.lock`의 ttc 버전을
  `0.3.0-dev.1`로 변경했다.
- 2026-08-23: release channel 판정이 `development`임을 확인하고 Node 테스트
  10건과 세 검증 게이트를 통과했다.

## 이슈 및 해결

없음.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

CI에서 TASK-180의 malformed projection panic이 확인되어 배포에 도달하지 못했다.
수정이 포함된 `0.3.0-dev.2`로 대체하므로 이 개발 릴리스는 취소한다.
