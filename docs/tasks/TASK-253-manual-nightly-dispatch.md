# TASK-253: main 수동 Nightly dispatch

- **상태**: 완료
- **시작일**: 2026-08-27
- **완료일**: 2026-08-27
- **커밋**: —

## 목적

예약 실행을 기다리지 않고 현재 `main`을 수동 CI로 검증한 뒤 동일한 Nightly 게시
경로로 승격할 수 있게 한다.

## 범위

- 포함: main `workflow_dispatch` CI의 Nightly 승격 허용, 검증 계약, 운영 문서
- 제외: 임의 CI run ID·npm tag 입력과 release branch 수동 게시 정책

## 의사결정

### 결정 1: Publish Release 자체가 아니라 main CI를 수동 실행한다

- **상황**: CI는 이미 `workflow_dispatch`를 지원하지만 후속 게시 gate는 예약 실행만
  Nightly로 인정한다.
- **검토한 대안**: 게시 workflow에 run ID 입력 / 로컬 npm publish / main 수동 CI를
  예약 CI와 같은 자동 승격 대상으로 인정.
- **선택과 근거**: 현재 main에서 새 불변 산출물을 만들고 기존 메타데이터 검증과
  최신-run 검사를 그대로 거치는 `workflow_dispatch` CI만 허용한다.

## 작업 내역

- 2026-08-27: 병합된 main을 pull하고 push CI 및 게시 workflow 상태를 확인했다.
- 2026-08-27: 수동 CI는 가능하지만 Publish Release가 event를 거부하는 원인을 확인했다.
- 2026-08-27: main 수동 CI를 예약 CI와 같은 Nightly 승격 대상으로 허용했다.
- 2026-08-27: 운영 문서와 workflow 계약 테스트를 갱신하고 전체 로컬 CI를 통과했다.

## 이슈 및 해결

### 이슈 1: 수동 CI 성공 뒤 Publish Release가 skipped 처리됨

- **증상**: main `workflow_dispatch` CI의 후속 `workflow_run`에서 verify job이 실행되지
  않는다.
- **원인**: verify 조건과 셸 검증이 main의 `schedule` event만 허용한다.
- **해결**: main의 `workflow_dispatch`만 Nightly 대상으로 허용했다. 기존 최신-run,
  소스 SHA·브랜치, 산출물 메타데이터 검증은 그대로 유지했다.

## 검증

- [x] 릴리스 workflow 계약 테스트
- [x] `./scripts/ci`
- [x] `git diff --check`

## 결과

현재 main을 수동 CI로 검증한 뒤 기존 자동 Nightly 게시 경로로 승격할 수 있다.
