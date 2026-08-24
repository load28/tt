# TASK-205: VS Code 전체 도구 체인 테스트 복구

- **상태**: 대기
- **시작일**: —
- **완료일**: —
- **커밋**: —

## 목적

TASK-204에서 확정한 match arm completion ICE를 구조적으로 제거하고, 현재 엔진
계약과 어긋난 확장 테스트를 정정해 전체 도구 체인 테스트를 0 실패·0 건너뜀으로
복구한다.

## 범위

- 포함:
  - 미완성 match arm body가 completion 전에 owner construction ICE를 내지
    않도록 parser-owned recovery 또는 probe 경계를 수정.
  - macOS lexical/canonical 경로를 동일 파일 identity로 비교하는 테스트 계약.
  - pipeline probe와 sidecar projection recovery의 현재 계약을 검증하는 테스트.
  - tsgo 주입 전체 확장 테스트와 저장소 필수 게이트.
- 제외:
  - 진단 억제나 문자열 휴리스틱으로 ICE를 숨기는 처리.
  - completion·sidecar의 사용자 계약 변경.

## 의사결정

(진행 시 기록)

## 작업 내역

(진행 시 기록)

## 이슈 및 해결

(진행 시 기록)

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] tsgo 주입 VS Code 확장 테스트 (0 실패·0 건너뜀)

## 결과

(진행 시 기록)
