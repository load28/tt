# TASK-216: exhaustiveness 수정을 컴파일러가 저작하는 편집으로

- **상태**: 대기
- **시작일**: —
- **완료일**: —
- **커밋**: —

## 목적

TASK-213이 진단에 `Suggestion { message, edit }` 채널을 만들고, 이름 오타는
적용 가능한 `Edit`으로 옮겼다. exhaustiveness는 아직 편집이 없는 조언
("add the missing arms or a final `_` arm")뿐이라, VS Code 확장이 빠진 태그
목록을 **진단 메시지의 렌더된 리스트에서 정규식으로** 읽고 arm 문자열도 직접
조립한다. 규칙 식별은 이미 `diag.code`로 옮겼지만, 태그 목록 파싱은 남아 있고
이는 AGENTS.md 계약 3이 금지하는 "문자열 모양에 기댄 해결"이다.

컴파일러가 arm 삽입 자체를 `Edit`으로 저작하면 그 정규식과 확장의 arm 조립이
함께 사라지고, CLI도 `= help:`에 붙여넣을 수 있는 텍스트를 보여줄 수 있다.

## 범위

- 포함:
  - `MatchAnalysis`에 match body의 닫는 위치를 싣는다(AST의
    `MatchExpr::body_close`는 이미 있다)
  - sema의 coverage 보고 경로가 소스 텍스트에 접근해 들여쓰기를 계산
  - 빠진 케이스의 필드 이름을 그 경로에서 얻는다 — 현재 `CoveredEnum`은
    이름과 origin만 갖는다
  - 확장에서 `MISSING_CASES_RE`와 `armFor` 제거
- 제외: 다른 규칙의 편집 저작 (필요할 때 각자의 태스크로)

## 의사결정

## 작업 내역

## 이슈 및 해결

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## 결과
