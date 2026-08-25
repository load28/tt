# TASK-218: 남은 규칙의 수정 조언을 `Suggestion`으로 옮기기

- **상태**: 대기
- **시작일**: —
- **완료일**: —
- **커밋**: —

## 목적

TASK-213 결정 2는 "메시지는 무엇이 잘못됐는지만 말하고, 고치는 법은
`Suggestion`이 싣는다"를 정했다. `unknown-case`, `unknown-field`,
`match-not-exhaustive`는 그렇게 옮겼지만 나머지 규칙은 아직 아니다. 지금은
규칙마다 조언의 위치가 다르다.

TASK-215의 픽스처가 그 불일치를 눈에 보이게 고정하고 있다 —
`diagnostic/stray-pipe`와 `diagnostic/let-else-not-diverging`의
`expected.stderr`에는 `= help:` 줄이 없고, 대신 조언이 메시지 괄호 안에 있다:

```
error[stray-pipe]: pipeline: `|>` could not be parsed here (steps must be
expressions; parenthesize ternaries and arrow functions)
```

결과적으로 소비자가 "이 진단의 수정 방법"을 한 곳에서 얻지 못한다. 에디터는
일부 규칙에만 quick fix를 줄 수 있고, `= help:` 줄의 유무가 규칙마다 다르다.

## 범위

- 포함:
  - 메시지 안에 수정 조언을 담고 있는 나머지 규칙을 조사해 목록화
  - 각각의 조언을 `TtError::help`(편집 없음) 또는 `TtError::suggest`(편집 있음)로
    이동. 적용 가능한 편집을 만들 수 있는 것은 편집으로 만든다
  - `docs/ai/tt.md`와 영향받는 테스트, `tests/fixtures/` 스냅샷 갱신
- 제외:
  - `DiagnosticCode::explanation`의 산문. 그것은 `ttc explain`의 긴 설명이지
    진단에 붙는 조언이 아니다.

## 의사결정

## 작업 내역

## 이슈 및 해결

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## 결과
