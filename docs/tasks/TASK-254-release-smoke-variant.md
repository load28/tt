# TASK-254: 릴리스 smoke test의 variant 전환

- **상태**: 완료
- **시작일**: 2026-08-27
- **완료일**: 2026-08-27
- **커밋**: —

## 목적

현재 tt 문법과 맞지 않는 릴리스 smoke 입력 때문에 검증된 Nightly 게시가 실패하는
문제를 해결한다.

## 범위

- 포함: 릴리스 smoke 입력의 `variant` 전환과 workflow 계약 테스트
- 제외: 컴파일러 문법 변경과 게시 절차 변경

## 의사결정

### 결정 1: smoke test도 공개 `variant` 문법을 사용한다

- **상황**: 게시 workflow가 과거 `enum` 기반 tt 구문을 사용해 현재 컴파일러 검증에서
  실패한다.
- **검토한 대안**: `--no-verify`로 우회 / 유효한 TypeScript만 통과 / 현재 tt 고유
  `variant` 문법으로 실제 변환 확인.
- **선택과 근거**: 검증을 유지하면서 공개 언어 표면의 변환까지 확인하는 `variant`를
  사용한다.

## 작업 내역

- 2026-08-27: 수동 Nightly 게시 run `33042537670`의 실패 로그를 확인했다.
- 2026-08-27: 릴리스 smoke 입력을 `variant`로 바꾸고 회귀 계약을 추가했다.
- 2026-08-27: 실제 컴파일 smoke와 전체 로컬 CI를 통과했다.

## 이슈 및 해결

### 이슈 1: 게시 전 compiler smoke test 실패

- **증상**: `enum E { A(x: number) }`에서 TypeScript 파싱 오류가 발생했다.
- **원인**: 태그드 유니언 선언이 `variant`로 전환됐지만 릴리스 workflow의 입력은
  과거 `enum` 문법에 남아 있었다.
- **해결**: 입력을 `variant E { A(x: number) }`로 바꾸고 계약 테스트가 과거 `enum`
  입력의 재도입을 거부하게 했다.

## 검증

- [x] 릴리스 workflow 계약 테스트
- [x] `./scripts/ci`
- [x] `git diff --check`

## 결과

게시 전 compiler smoke test가 현재 tt 문법으로 실제 변환을 검증한다.
