# TASK-216: exhaustiveness 수정을 컴파일러가 저작하는 편집으로

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: (아래 "작업 내역")

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

### 1. 필드 이름은 `CoveredEnum`이 아니라 **witness**에서 얻는다

범위는 `CoveredEnum`(이름 + origin)에 필드를 붙이는 것을 예상했지만, 그 자리는
필요 없었다. exhaustiveness가 만들어내는 `Witness::Ctor { tag, args }`의 `args`가
이미 **선언된 모든 필드**를 순서대로 담고 있다("Every declared field, in order,
with what the witness needs it to be. Rendering drops the wildcards"). 메시지가
`Circle`만 쓰는 것은 렌더러가 wildcard를 버리기 때문이지, 이름을 모르기
때문이 아니었다.

그래서 `Witness`에 두 번째 읽기를 더했다.

| 읽기 | 답하는 질문 | `Circle(r: number)`가 미커버일 때 |
|---|---|---|
| `render()` | 어떤 값이 처리되지 않았나 | `Circle` |
| `arm()` | 그것을 처리하려면 뭐라고 쓰나 | `Circle(r)` |

두 읽기가 같은 witness에서 나오므로 필드 이름의 두 번째 출처가 생기지 않는다.
`CoveredEnum`은 그대로 두었다 — 타입 경로에서는 어차피 `positions: vec![None]`
(선언이 아니라 체커의 알파벳에서 답이 나온다)이므로, 거기 필드를 실었어도
타입 경로는 답을 얻지 못했을 것이다.

`arm()`이 바인딩을 쓰는 것은 **arm 자신의 레벨뿐**이다. 그보다 깊은 곳에서
`field: Name`은 중첩 패턴을 쓰는 문법이므로(`language.md` §3.2), 거기서
바인딩을 넣으면 패턴이 검사하는 대상이 바뀐다. 중첩 위치는 `render()`의 형태를
유지한다.

### 2. 편집 저작은 진단 계층 하나에, 두 파이프라인이 공유

`match-not-exhaustive`를 만드는 곳은 셋이다: 기본 경로(`sema::report_coverage`),
타입 경로의 태그 소진성, 타입 경로의 리터럴 유니온 소진성. 편집을 각자
만들면 세 벌이 어긋난다. `src/diagnostics.rs`에 `MatchSite`(키워드 + body 중괄호
두 개)와 `non_exhaustive_suggestions()`를 두고 셋 다 그것을 부른다.

`MatchSite`가 성립하려면 세 경로 모두 body 중괄호를 갖고 있어야 한다.

- 기본 경로: `MatchAnalysis`에 `body_open`/`body_close`를 추가(AST가 이미 갖고
  있다).
- 타입 경로: 보고가 파스가 아니라 **체커의 답**에서 나오므로, 문법 사실이
  질문과 함께 이동해야 한다 — `TagMatch`/`LiteralMatch`(방출이 기록하는 프로브)에
  두 오프셋을 싣고, projection의 앵커를 `MatchAnchor`로 올렸다.

### 3. 한 진단의 여러 `Suggestion`은 **대안**이다

"arm들을 쓴다"와 "`_` arm을 쓴다"는 둘 다 적용하면 안 되는 두 가지 해법이다.
`Suggestion`의 문서가 이미 "One way to resolve a `Diagnostic`"이라고 말하고
있으므로 모델을 바꿀 필요는 없었지만, 테스트 헬퍼가 한 진단의 편집을 전부
적용하고 있었다(편집이 최대 하나였을 때는 같은 뜻이었다). 헬퍼를
`with_suggestion_applied(source, diagnostic, which)`로 바꿔 계약을 코드에
드러냈다. 확장은 원래부터 액션을 따로 올리므로 변화가 없다.

### 4. 삽입 위치는 body의 레이아웃을 따른다

`}`가 자기 줄에 있으면 그 줄 위에 완전한 arm 줄들을 넣고, 한 줄짜리 match면
마지막 arm 뒤에 이어 붙인다. 후자의 편집 범위는 `}` 앞이 아니라 **본문 텍스트가
끝나는 곳부터 `}`까지**다 — `{ Empty => 0 }`에서 `}` 앞에 붙이면
`Empty => 0 , Circle(r) => undefined, }`처럼 공백이 가운데 남는다.

들여쓰기는 `match` 키워드가 있는 줄의 선행 공백 + 2칸이다. 사용자 코드를
읽기만 하고 다시 쓰지 않으므로 계약 1과 충돌하지 않는다.

### 5. 확장의 액션 제목은 컴파일러의 문장

기존 제목은 편집 텍스트로 만들었다(`` `Circle`(으)로 바꾸기 ``). 삽입되는 arm
블록은 제목이 될 수 없으므로, 제목을 `suggestion.message`로 바꿨다. 그 결과
확장의 code action 처리기에는 규칙별 분기가 하나도 남지 않는다.

### 6. 여러 줄 편집은 렌더러가 코드처럼 그린다

`= help: ...: \`텍스트\`` 형태는 한 줄 대체에는 맞지만 arm 블록에는 맞지 않는다.
줄바꿈이 있는 편집은 스니펫과 같은 `|` 접두로 그려, 읽는 사람이 파일에 들어갈
모습 그대로 보게 했다. 편집의 바이트 자체는 바뀌지 않는다 — 그림만 다듬는다.

## 작업 내역

1. `src/analysis/usefulness.rs`: `Witness::arm()` 추가 — 제약이 없는 필드를
   버리지 않고 그 이름으로 바인딩하는 arm 패턴 렌더링.
2. `src/analysis/mod.rs`: `Uncovered`에 `arm: Vec<String>` 추가,
   `render_witnesses`가 두 형태를 함께 만든다. `MatchAnalysis`에 `body_open`,
   `body_close` 추가(단일·튜플 양쪽).
3. `src/diagnostics.rs`: `MatchSite`, `non_exhaustive_suggestions()`,
   `insert_arms()` 추가. `NON_EXHAUSTIVE_HELP`를 "add the missing arms"로 줄이고
   `NON_EXHAUSTIVE_WILDCARD_HELP`를 새로 뒀다.
4. `src/sema.rs`: `check_all`/`report_coverage`가 소스 텍스트를 받고, 커버리지
   진단이 두 개의 편집을 싣는다. `src/lib.rs`의 호출을 맞췄다.
5. `src/probe.rs`: `LiteralMatch`/`TagMatch`에 body 중괄호 오프셋 추가.
   `src/engine/projection.rs`: `MatchAnchor` 도입.
   `src/engine/semantics.rs`: 태그·리터럴 두 경로가 같은 저작기를 부른다.
6. `src/render.rs`: 여러 줄 편집을 `|` 접두 코드 블록으로, 한 줄 편집은
   기존대로 백틱으로 그린다.
7. `editors/vscode/server/src/server.ts`: `MISSING_CASES_RE`, `armFor`,
   `insertArms`, code action 경로의 `declarationsOf` 호출을 모두 삭제.
   `onCodeAction`은 이제 `suggestedFixes` 하나만 부른다(−100줄).
8. 테스트: `tests/compile.rs`에 저작된 arm의 계약 5개(내용, 페이로드 바인딩,
   적용하면 소진성이 해소됨, 와일드카드 대안, 한 줄 match 유지). 확장 테스트
   2개를 새 제목·새 편집으로 갱신.
9. 픽스처 갱신 후 diff 검토, `docs/why-tt.md`·`docs/why-tt.ko.md`·
   `docs/design/type-inference-gaps.md`·`website/src/essay.json`의 예시 출력 갱신.

## 이슈 및 해결

- **증상**: `applying_the_authored_arms_makes_the_match_exhaustive`가 arm 편집과
  와일드카드 편집을 **둘 다** 적용해 `Empty => 0 , _ => undefined, , Circle...`를
  만들었다.
- **원인**: 테스트 헬퍼가 한 진단의 모든 편집을 적용하고 있었다. 편집이 최대
  하나일 때는 옳았지만, 대안 두 개가 생기자 틀린 모델이 됐다.
- **해결**: 결정 3 — 헬퍼가 하나의 suggestion만 적용한다.

- **증상**: 한 줄 match에 붙인 arm 앞에 공백이 남았다(`Empty => 0 , Circle...`).
- **원인**: 편집 범위를 `}` 위치의 길이 0 삽입으로 잡아, `}` 앞의 패딩이 arm
  앞으로 밀렸다.
- **해결**: 결정 4 — 본문 텍스트가 끝나는 곳부터 `}`까지를 대체한다.

- **증상**: 빈 body(`match (v) { }`)를 테스트 케이스로 넣었더니 진단이 아예
  나오지 않았다.
- **원인**: arm이 없는 `match (v) { }`는 tt match가 아니라 TypeScript로 읽히고,
  `source-not-typescript`로 보고된다. tt 구문이 아닌 것에 tt 규칙을 기대한
  테스트가 잘못이었다.
- **해결**: 케이스를 튜플 match와 페이로드 홀로 교체했다 — 두 경우 모두 저작된
  arm이 소진성을 실제로 해소하는 것을 확인했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 전체 초록
- [x] `UPDATE_EXPECT=1 cargo test --test snapshot` 후 diff 검토
- [x] VS Code 확장 `server.test.ts` 17/17 통과 (`TTC_TSGO_ROOT` 설정, skip 0)

## 결과

`match-not-exhaustive`의 수정이 데이터가 됐다. CLI는 붙여넣을 수 있는 arm
텍스트를 보여주고, `--server` JSON은 두 개의 `edit`을 싣고, VS Code는 그것을
그대로 적용한다. 확장에서 진단 메시지를 읽는 코드는 남아 있지 않다.

리터럴 유니온 소진성도 이제 편집을 갖는다. 이전 확장의 정규식 경로는 이
경우에 `"a"` 대신 `a =>`를 만들어 **잘못된 arm을 제안하고 있었다** — 규칙별
분기를 지운 부수 효과로 함께 고쳐졌다.

### 변경 파일

- `src/analysis/usefulness.rs`, `src/analysis/mod.rs`
- `src/diagnostics.rs`, `src/sema.rs`, `src/lib.rs`, `src/render.rs`
- `src/probe.rs`, `src/engine/projection.rs`, `src/engine/semantics.rs`
- `tests/compile.rs`
- `tests/fixtures/diagnostic/match-not-exhaustive/expected.{stderr,json}`
- `tests/fixtures/diagnostic/many-in-one-file/expected.{stderr,json}`
- `editors/vscode/server/src/server.ts`
- `editors/vscode/server/src/test/server.test.ts`
- `docs/why-tt.md`, `docs/why-tt.ko.md`, `docs/design/type-inference-gaps.md`
- `website/src/essay.json`
- `docs/tasks/INDEX.md`
