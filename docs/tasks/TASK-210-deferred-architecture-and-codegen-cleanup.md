# TASK-210: 잔여 아키텍처·codegen 개선 묶음

- **상태**: 대기
- **시작일**: —
- **완료일**: —
- **커밋**: —

## 목적

현재 소비자가 없어 보류됐거나 사용자 동작을 바꾸지 않는 잔여 아키텍처·codegen
개선 후보를 한 태스크에서 관리하고, 추후 함께 착수한다.

## 범위

- 포함:
  - Phase 6 query 세분화(`pattern_analysis`/`flow_body` 단위)
  - 불필요한 receiver temporary 제거와 직접 호출 가능한 `$tt_ap` 최적화
  - 생성 코드의 `do { ... } while (false)`·레이블 블록 이중 중첩 단일화와 임시 이름 정리
  - template interpolation recovery를 두 번 방문하는 parser traversal 중복 제거
- 제외: TASK-209의 output verify 휴리스틱 제거, 새 언어 표면, 필요성이 확인되지 않은 IR 확장

## 의사결정

(착수 시 기록)

## 작업 내역

(착수 시 기록)

## 이슈 및 해결

(착수 시 기록)

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## 결과

(착수 시 기록)
