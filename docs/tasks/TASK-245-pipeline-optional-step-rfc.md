# TASK-245: Discussion #64 파이프라인 optional 스텝 RFC 정리

- **상태**: 완료
- **시작일**: 2026-08-26
- **완료일**: 2026-08-26
- **커밋**: —

## 목적

Discussion #64의 숙의 결과(선택지 A 채택, 전체 postfix tail, 공유
`push_receiver`, 검증 행렬)가 라운드별 댓글에만 흩어져 있어 구현이 참조할 규범이
없다. 결정을 저장소 설계 문서 하나로 정리한다.

## 범위

- 포함: Discussion #64의 확정 문법·방출·의미·진단·완료 조건을 설계 문서로 정리,
  `pipeline-operator.md`의 stale한 `?.` 항목 갱신, 숙의 경과 요약 보존
- 제외: optional 스텝 구현(parser·HIR·codegen·sema), `docs/ai/tt.md`와 README의
  언어 표면 갱신, Discussion 본문 편집 — 모두 구현 태스크 몫

## 의사결정

### 결정 1: 파이프라인 설계 문서를 고치지 않고 별도 문서로 분리한다

- **상황**: 결정을 `pipeline-operator.md`에 흡수할지, 새 문서로 둘지 골라야 했다.
- **검토한 대안**: `pipeline-operator.md` §3.2 확장 / 새 설계 문서 /
  태스크 문서에만 기록
- **선택과 근거**: 새 문서(`docs/design/pipeline-optional-steps.md`)로 둔다.
  `pipeline-operator.md`는 **구현됨** 상태의 규범이고 이 결정은 **미구현
  확정안**이라, 한 문서에 섞으면 어느 문장이 현재 컴파일러 동작인지 읽는 사람이
  구분할 수 없다. 기존 문서에는 새 문서를 가리키는 한 항목만 남긴다.

### 결정 2: 라운드별 발언은 규범에서 분리해 경과로 압축한다

- **상황**: 숙의가 4라운드 + 3라운드였고 두 관점이 중간에 입장을 철회했다. 발언을
  그대로 옮기면 철회된 제안이 규범처럼 읽힌다.
- **검토한 대안**: 댓글 전문 전재 / 최종 결론만 기록 / 규범 + 경과 분리
- **선택과 근거**: §2~§7은 확정 계약만 담고, 라운드별 움직임은 §8 표로 압축한다.
  "제한 제안이 왜 철회됐는지"는 구현이 같은 제안을 반복하지 않게 하는 근거이므로
  버리지 않고 §3의 근거 단락과 §8에 남긴다.

### 결정 3: Discussion 본문은 편집하지 않는다

- **상황**: "정리"의 대상이 GitHub Discussion 본문일 수도, 저장소 문서일 수도
  있었다.
- **검토한 대안**: Discussion 본문·댓글 편집 / 저장소 설계 문서 정리
- **선택과 근거**: 저장소 문서로 정리한다. Discussion은 숙의 기록 그대로가
  증거이고, 구현이 참조하는 단일 진실 소스는 저장소 문서여야 한다. 세션의 GitHub
  도구에도 Discussion 쓰기 경로가 없다.

## 작업 내역

- 2026-08-26: Discussion #64의 본문, 세 관점의 1~4라운드 발언, 진행자 합의 2건을
  읽고 확정 계약과 철회된 제안을 분리했다.
- 2026-08-26: `src/codegen/core.rs`의 `push_receiver`(2890행)와 `emit_apply`의
  `ApplyMode::Postfix` 경로를 읽어, 합의가 말하는 "공유 판정기"의 실제 동작
  (확정된 primary expression만 괄호 생략, 미확정 lowering은 괄호 유지)을 문서
  기술과 맞췄다.
- 2026-08-26: `docs/design/pipeline-optional-steps.md`를 작성했다 — 선택지 비교,
  확정 문법(EBNF), 방출 계약과 입력→출력 표, 의미 계약, 진단, 검증 행렬,
  숙의 경과, 후속 작업.
- 2026-08-26: `docs/design/pipeline-operator.md` §3.2의 "`?.` 시작은 1차 범위에서
  제외한다" 항목을 새 문서 링크와 미구현 상태 표기로 갱신했다.
- 2026-08-26: `docs/tasks/INDEX.md`에 TASK-245를 등록하고 다음 번호를 246으로
  올렸다.

## 이슈 및 해결

### 이슈 1: 스레드의 예시 출력이 확정 방출 규칙과 어긋난다

- **증상**: 초기 라운드 예시는 `user |> ?.profile`을 `(user)?.profile`로 적었지만,
  논점 2의 합의는 확정 primary 수신자의 괄호를 생략한다.
- **원인**: 괄호 정책 합의(논점 2)가 문법 합의(논점 1)보다 **뒤에** 나왔고,
  앞 라운드 예시가 갱신되지 않았다.
- **해결**: 설계 문서의 방출 표는 나중 합의를 기준으로 `user?.profile`로 적고,
  `(E)?.tail`은 괄호 판정 이전의 개념적 형태로만 기술했다.

## 검증

- [x] 문서 전용 변경 — Rust 소스 변경 없음
- [x] 새 문서와 갱신 항목의 상대 링크 대상 존재 확인
  (`pipeline-operator.md`, `TASK-245`, `docs/design/` 상호 링크)
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

- 추가: `docs/design/pipeline-optional-steps.md`
- 수정: `docs/design/pipeline-operator.md`, `docs/tasks/INDEX.md`

Discussion #64의 결정이 구현이 따를 수 있는 규범 형태로 남았다. 실제 구현
(parser·HIR·codegen·sema·테스트·언어 문서 갱신)은 §9의 후속 작업 목록대로 별도
태스크로 등록한다.
