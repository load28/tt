# TASK-215: 스냅샷 픽스처 — 방출과 진단을 통째로 고정한다

- **상태**: 진행 중
- **시작일**: 2026-08-25
- **완료일**: —
- **커밋**: —

## 목적

`tests/compile.rs`의 어서션은 `contains(` 479개 대 `assert_eq!` 175개다. 즉 방출
결과를 **부분 문자열**로만 확인한다. 그 결과 TASK-198이 정한 "방출 코드 가독성 —
레이아웃 계층과 그룹핑 규칙"을 지키는 게이트가 없다. 여분의 문장, 깨진 들여쓰기,
중복된 헬퍼는 전부 `contains("switch ($tt_m.kind)")`를 통과한다.

TASK-213이 만든 렌더된 진단도 같은 문제를 갖는다. 캐럿 한 칸, `= help:` 한 줄이
어긋나도 `contains("not exhaustive")`는 통과한다.

이 태스크는 **전체 산출물을 파일로 고정**한다. diff가 곧 리뷰 대상이 되어, 방출
품질과 진단 표현이 눈에 보이는 것이 된다 — rustc의 `tests/ui`가 하는 일이다.

## 범위

- 포함:
  - `tests/fixtures/`와 이를 걷는 러너 `tests/snapshot.rs`
  - `UPDATE_EXPECT=1`로 기대 파일 갱신
  - 방출: `expected.ts` — 컴파일 결과 전체
  - 진단: `expected.stderr`(CLI 렌더 결과 전체)와 `expected.json`(`--server`
    와이어 포맷 전체). 둘 다 고정해야 두 출구가 드리프트하지 않는다
  - 각 구문과 각 진단 계열을 덮는 초기 픽스처 집합
- 제외:
  - `tests/compile.rs`의 479개 `contains` 어서션을 일괄 이관하는 것. 그 파일의
    많은 테스트는 "이 한 가지 성질"을 검사하는 단위 테스트이고 스냅샷이 대체할
    대상이 아니다. 스냅샷은 **산출물 전체**가 계약인 곳에 쓴다.

## 의사결정

## 작업 내역

## 이슈 및 해결

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## 결과
