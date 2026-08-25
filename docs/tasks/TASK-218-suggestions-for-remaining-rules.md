# TASK-218: 남은 규칙의 수정 조언을 `Suggestion`으로 옮기기

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: (아래 "작업 내역")

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

### 1. "조언"의 경계 — 고치는 법만 옮기고, 사실과 근거는 메시지에 남긴다

메시지 안 괄호를 전부 `help:`로 옮기면 규칙은 단순하지만 잘못된다. 진단
메시지의 괄호 안에는 세 가지가 섞여 있었다.

| 종류 | 예 | 처리 |
|---|---|---|
| **수정 조언** — 사용자가 무엇을 쓰면 되는지 | `parenthesize ternaries and arrow functions` | `help`/`suggest`로 이동 |
| **사실** — 컴파일러가 무엇을 봤는지 | `found string after number`, `has 2 elements but the match has 1 scrutinees` | 메시지에 유지 |
| **근거** — 왜 규칙이 있는지 | `the binding is declared with val, so every access path from it is read-only` | 메시지에 유지 |

`help:`는 rustc에서 "이렇게 고쳐라"의 자리다. 사실이나 근거를 거기 넣으면
소비자가 "help의 내용을 적용하면 에러가 사라진다"는 성질을 잃는다 — 그
성질이 quick fix를 만들 수 있는지 없는지를 가르는 기준이므로, 경계는 편집
가능성 쪽에 맞춘다.

이 기준으로 `val-mutation`·`val-pass`의 괄호(근거),
`match-or-literal-kind-mismatch`·`match-tuple-arity`·`match-or-binding-mismatch`의
괄호와 대시 뒤(사실), `enum-invalid-field-type`의 원인 문구(사실)는 그대로 뒀다.

### 2. 조언이 아예 없던 규칙에는 조언을 새로 만들지 않는다

`match-wildcard-not-last`, `match-duplicate-arm`, `enum-duplicate-case`,
`match-nested-in-or-pattern`은 메시지가 규칙을 그대로 진술하고(`_` 암은 마지막
이어야 한다), 조언을 따로 갖고 있지 않았다. 이 태스크는 **조언의 자리를
통일**하는 것이지 조언을 늘리는 것이 아니므로 건드리지 않았다. 각 규칙의 긴
설명은 이미 `ttc explain <code>`에 있다.

### 3. `malformed-match`의 두 갈래를 하나의 메시지 + 두 종류의 조언으로

`match value { ... }`(스크루티니가 식별자로 시작)와 그 밖의 파싱 실패는 지금까지
**서로 다른 메시지**를 냈다. 규칙 하나가 메시지 두 개를 갖는 것은 소비자에게
규칙이 둘인 것처럼 보인다. 메시지를 `tt \`match\` could not be parsed` 하나로
합치고, 갈래는 조언으로 표현했다.

식별자 갈래는 조언이 **편집**이 된다: 키워드와 본문 `{` 사이의 텍스트가 곧
스크루티니 식이고 괄호만 없으므로, `[식별자 시작, 텍스트 끝)`를 `(텍스트)`로
바꾸는 `Edit` 하나로 정확히 표현된다. 파서가 이미 갖고 있는 위치 정보만
쓰므로 문자열 모양 추정이 아니다. 나머지 갈래는 쓸 수 있는 텍스트가 없으므로
편집 없는 `help`다.

### 4. `result-missing-keyword`도 편집으로

`b <- f();`에서 빠진 것은 바인딩 앞의 선언 키워드 하나다. 파서가 이미 바인딩
이름의 전체 span을 싣고 있으므로 `span.start`에 `const `를 삽입하는 길이 0
범위의 `Edit`이 정확한 표현이다. `let`/`var`도 같은 자리에 오지만 편집은 하나만
실을 수 있으므로 가장 흔한 `const`를 쓰고, 나머지는 조언 문장이 이름을 부른다.

## 작업 내역

1. `DiagnosticCode` 30개를 모두 훑어 메시지에 조언이 섞인 규칙을 목록화했다.
   대상 14개: `stray-pipe`, `stray-if-let`, `stray-result`,
   `result-missing-keyword`, `result-nested-binding`, `flow-first-step-method`,
   `try-placement`(sema 2갈래 + parser 1갈래), `let-else-placement`,
   `let-else-not-diverging`, `if-let-placement`, `pattern-duplicate-binding`(2곳),
   `match-mixed-patterns`, `malformed-match`, `malformed-enum`.
2. `src/sema.rs`의 12개 보고 지점에서 조언을 `.help(...)`로 분리했다.
   `try-placement`는 두 메시지가 각각 다른 조언을 갖도록 `(message, help)`
   쌍으로 바꿨다.
3. `result-missing-keyword`를 `.suggest(...)`로 바꿔 `const ` 삽입 편집을 실었다.
4. `src/parser/enums.rs`(`malformed-enum`), `src/parser/mod.rs`(식 위치의 `try`)의
   조언을 `.help(...)`로 분리했다.
5. `src/parser/matches.rs`의 `malformed-match`를 메시지 하나로 합치고, 식별자
   갈래에 괄호 삽입 `Edit`을 실었다. 본문 `{`의 토큰 인덱스를 한 번만 찾아
   진단 범위와 편집 범위가 같은 사실에서 나오도록 했다.
6. `tests/compile.rs`에 `advice(src)` 헬퍼(모든 진단의 `help` 문장)를 추가하고,
   메시지에서 조언을 읽던 4개 테스트를 조언 채널에서 읽도록 고쳤다.
   `malformed-match`는 편집을 적용한 결과까지 고정했다.
7. `tests/native.rs`, `editors/vscode/server/src/test/server.test.ts`의 같은
   가정을 갱신했다. 확장 테스트는 이제 quick fix가 실제로 만들어지는지를
   (`data.suggestions[0].edit.replacement === "(shape)"`) 확인한다.
8. `docs/design/pipeline-operator.md`의 예시 출력을 현재 렌더 형태로 갱신했다.
9. `UPDATE_EXPECT=1 cargo test --test snapshot`으로 픽스처를 갱신하고 diff를
   검토했다 — `stray-pipe`와 `let-else-not-diverging` 두 케이스에서 조언이
   메시지에서 `= help:` 줄로, JSON에서는 `suggestions[]`로 옮겨졌다.

## 이슈 및 해결

- **증상**: `cargo test --test native`의
  `parser_errors_do_not_hide_an_independent_type_error_in_the_same_file`이
  `\`match\` could not be parsed here`를 찾지 못해 실패.
- **원인**: 결정 3으로 `malformed-match`의 식별자 갈래 메시지가 사라지고 공통
  메시지 + 조언으로 바뀌었다. 테스트가 옛 갈래별 메시지를 문자열로 보고 있었다.
- **해결**: 테스트를 공통 메시지와 조언 문장 양쪽을 확인하도록 고쳤다. 진단이
  "보인다"는 계약은 그대로이고, 무엇을 보고 확인하느냐만 새 채널로 옮겼다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 전체 초록 (unit 225, compile 332, integration 99, native 40,
      passthrough 57, snapshot 4, 나머지 스위트 포함)
- [x] `UPDATE_EXPECT=1 cargo test --test snapshot` 후 diff 검토

## 결과

모든 tt 규칙의 수정 조언이 한 채널(`Diagnostic::suggestions`)에 있다. CLI는
어느 규칙이든 `= help:` 줄로 같은 자리에 그리고, `--server` JSON은 같은
`suggestions[]` 배열로 싣고, VS Code 확장은 편집이 있는 조언을 quick fix로
올린다. 편집을 실을 수 있는 규칙이 셋(`unknown-case`, `unknown-field`)에서
다섯(`malformed-match`, `result-missing-keyword` 추가)으로 늘었다.

### 변경 파일

- `src/sema.rs`
- `src/parser/matches.rs`
- `src/parser/enums.rs`
- `src/parser/mod.rs`
- `tests/compile.rs`
- `tests/native.rs`
- `tests/fixtures/diagnostic/stray-pipe/expected.{stderr,json}`
- `tests/fixtures/diagnostic/let-else-not-diverging/expected.{stderr,json}`
- `editors/vscode/server/src/test/server.test.ts`
- `docs/design/pipeline-operator.md`
- `docs/tasks/INDEX.md`
