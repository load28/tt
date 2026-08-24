# TASK-198: 방출 코드 가독성 — 레이아웃 계층과 그룹핑 규칙

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: `ff7e301`

## 목적

ttc가 방출한 TypeScript를 사람이 읽기 어려웠다. 두 가지 원인이 있었다.

1. **글루가 열 0에 붙었다.** 로프에는 줄바꿈이 리터럴 `"\n"`으로만 들어 있고
   그 뒤에 들여쓰기를 쓰는 주체가 없어서, 함수 본문 안에서 lower된 구문의
   블록 구조가 통째로 파일 왼쪽 끝에 붙었다:

   ```ts
   export function area(s: Shape): number {
     let $tt_v0;
   {
     const $tt_m = (s);
     switch ($tt_m.kind) {
       case "Circle": { const { radius } = $tt_m; $tt_v0 = (Math.PI * radius * radius); break; }
   ...
   }
   return $tt_v0;
   }
   ```

   `switch`/`case`의 상대 들여쓰기는 `"  "`, `"    "` 같은 **고정 문자열**로
   방출부에 하드코딩돼 있었기 때문에, 구문이 어느 열에 놓이든 항상 같은
   자리에서 시작했다.

2. **의미 없는 괄호가 값마다 붙었다.** `$tt_v0 = (0);`, `const $tt_m = (s);`,
   `$tt_ap((xs), (f))`, `if ((radius > 10))` 처럼 lower된 값은 예외 없이
   괄호로 감싸여 방출됐다.

둘 다 생성 코드를 읽을 때(디버깅, 번들 확인, 리뷰) 그대로 비용이 된다.

## 범위

- 포함: 로프에 레이아웃 개념(줄바꿈 + 들여쓰기 범위) 도입, 방출부 전체를 그
  위로 이전, 값 위치의 괄호 필요성 판정(스캐너 술어 2개), 회귀 테스트,
  설계·AI 문서 갱신
- 제외: verbatim 원문 재포맷(계약 1이 금지), 임시 이름 체계 변경
  (`$tt_v0`/`$tt_m`/`$tt_y_$tt_v1`), `do { … } while (false)`/레이블 블록의
  중첩 구조 변경, block arm의 `= undefined` 폴스루 제거

## 의사결정

### 결정 1: 레이아웃은 프린터가 소유한다 — 방출부는 "구조"만 말한다

- **상황**: 들여쓰기를 넣으려면 각 방출 지점이 자기가 인쇄될 열을 알아야
  한다. 그런데 로프 조각은 독립적으로 만들어져 나중에 조립되므로, 만드는
  시점에는 열을 알 수 없다.
- **검토한 대안**:
  - **A. 방출부에 base indent를 인자로 흘린다.** 각 `emit_*`가 문자열
    들여쓰기를 직접 계산한다. 명시적이지만 20여 개 함수의 시그니처를 바꾸고,
    새 방출 코드마다 이 인자를 잊지 않아야 한다 — 규칙이 코드에 흩어진다.
  - **B. 평탄화 후 텍스트를 재들여쓰기한다.** 출력 텍스트에서 중괄호 깊이를
    세어 생성 줄만 다시 쓴다. 원문 조각의 중괄호까지 세야 해서 문자열
    휴리스틱이 되고(CLAUDE.md 불변 원칙 3 위반), 매핑 오프셋도 전부 흔든다.
  - **C. 로프에 레이아웃 조각을 넣고 프린터가 해석한다.** 방출부는
    `push_break(depth)`("여기서 줄을 끝내고 depth만큼 안쪽에서 다시 시작")만
    말하고, 실제 공백은 인쇄 시점에 정해진다.
- **선택과 근거**: **C**. 방출부는 자기가 만드는 **블록 구조**는 알지만 인쇄
  열은 모른다 — 아는 것만 말하게 하는 배치다. 들여쓰기 규칙이 프린터
  (`codegen/rope.rs`) 한 곳에 있으므로 새 구문이 추가돼도 규칙이 복제되지
  않는다. 기존의 하드코딩된 `"  "`/`"    "`가 그대로 depth 1/2로 번역돼
  이전 비용도 낮았다.

### 결정 2: base는 "스코프가 열린 줄의 들여쓰기", 스코프는 anchor가 연다

- **상황**: `push_break(depth)`가 실제 공백이 되려면 기준(base)이 필요하다.
- **검토한 대안**:
  - **A. 스코프가 열린 지점의 열(column)**. `const label = match …`처럼 줄
    중간에서 시작하는 구문은 base가 14열이 돼 블록이 과도하게 밀린다.
  - **B. 스코프가 열린 줄의 선행 공백**. 그 문장이 놓인 들여쓰기와 정확히
    같은 자리에서 블록이 시작한다.
- **선택과 근거**: **B**. 사람이 손으로 쓸 때와 같은 규칙이다. 탭 들여쓴
  소스를 위해 폭(width)이 아니라 **선행 공백 문자열 자체**를 복사한다.
  스코프를 여는 지점은 `Rope::anchored` — 이미 "이 글루는 이 구문의 것"을
  표시하는 유일한 경계이므로, "구문의 글루는 자기가 시작한 줄에서
  레이아웃된다"는 규칙이 별도 호출 없이 모든 구문에 한 번에 적용된다.
  중첩 구문은 자기 스코프를 다시 열어 그 시점 줄을 기준으로 삼는다.

### 결정 3: 조각의 깊이는 상대값, 중첩은 `Rope::indented`가 옮긴다

- **상황**: `emit_switch`는 자기 기준 depth 0/1에서 조립되지만, 그 결과는
  `emit_value_decision`의 depth 1 안쪽에 들어간다.
- **검토한 대안**: 호출자가 depth를 인자로 내려보내기 / 조립 시점에 옮기기.
- **선택과 근거**: 후자(`Rope::indented(1, fragment)`). 각 방출 함수는 자기
  단독 기준으로만 작성되고, 중첩은 append하는 쪽이 표현한다. `indented`는
  **자기 스코프의 break만** 옮기고 조각이 새로 연 스코프 안쪽은 건드리지
  않는다 — 그 안쪽은 자기 base를 따로 갖기 때문이다.

### 결정 4: 괄호는 "재결합될 수 있을 때만" 남긴다

- **상황**: 값 위치의 괄호를 무조건 지우면 의미가 바뀔 수 있다. 무조건
  남기면 지금의 잡음이 그대로다.
- **검토한 대안**:
  - **A. 값이 원자적일 때(단일 토큰/괄호쌍)만 지운다.** 안전하지만
    `$tt_v0 = (w * h)` 같은 대다수 경우를 못 지운다.
  - **B. 위치가 무엇을 재결합할 수 있는지로 판정한다.** 초기화식,
    대입 우변, `return` 피연산자, 인자 하나 — 이 위치들에서 자기보다 느슨하게
    묶이는 연산자는 **콤마 하나뿐**이다. 그래서 "최상위 콤마가 있으면 남기고
    없으면 지운다"가 이 위치들의 완전한 규칙이다.
- **선택과 근거**: **B**. 위치별 특례가 아니라 우선순위라는 하나의 근거에서
  나오는 규칙이고, 판정은 스캐너의 술어(`has_top_level_comma`) 하나로
  끝난다. postfix 수신자(`(await p).then(g)`)는 다른 질문(멤버 접근보다
  느슨한 게 있는가)이므로 별도 술어(`is_primary_expression`)로 답한다.
  두 술어 모두 애매하면 **괄호를 남기는 쪽**으로 답하도록 썼다 —
  틀리면 괄호 한 쌍을 손해 보고, 의미는 절대 잃지 않는다.

### 결정 5: verbatim 원문은 재포맷하지 않는다

- **상황**: match arm 본문 같은 원문 블록은 소스의 들여쓰기를 그대로 갖고
  들어오므로, 깊어진 방출 위치와 어긋나 보일 수 있다.
- **선택과 근거**: 계약 1("유효한 TS는 바이트 그대로 통과")과 원문↔출력 매핑의
  정확성이 재포맷보다 우선한다. 레이아웃은 **컴파일러가 쓴 글루에만**
  적용한다. 사용자가 쓴 줄은 사용자가 쓴 모양으로 남는다.

## 작업 내역

- 2026-08-24: 현상 재현. `ttc -p`로 match/try/`result`/파이프라인/let-else/
  `if let`을 방출해 열 0 문제와 괄호 잡음을 확인.
- `src/codegen/rope.rs`: `Piece::Break { depth }`, `Piece::ScopeOpen/ScopeClose`
  추가. 프린터(`TargetFile::print`)가 스코프 스택을 들고 break를 "개행 +
  base + `INDENT`×depth"로 해석한다(`line_indent`가 base를 읽는다). `Rope`에
  `push_break`/`scoped`/`indented`/`resolved_text` 추가. `trim`,
  `ends_with_newline`, `last_line_has_line_comment`가 break를 공백/줄 경계로
  다루도록 확장. `anchored`가 글루를 스코프로 감싼다.
- `src/codegen/core.rs`: 리터럴 `"\n"`과 하드코딩 들여쓰기를 전부 break로 이전
  (`emit_value_decision`/`emit_switch`/`emit_if_chain`/`emit_arm_action`/
  `emit_result_region_continued`/`emit_apply_continued`/`emit_scheduled_step`/
  owner slot·compose·arrow return rewrite). `ArmEmissionContext.indent:
  &'static str` → `depth: u16`. `unexpected_switch`/`unexpected_throw`/
  `guard_line_comment`/`emit_value_delivery`가 레이아웃을 인자로 받는다.
  `emit_adt_text`(String) → `emit_adt`(Rope)로 바꿔 중첩 enum 선언도
  자기 줄에서 레이아웃된다.
- `src/scanner.rs`: `has_top_level_comma`, `is_primary_expression` 추가.
- `src/codegen/core.rs`: `push_grouped`/`push_receiver`/`needs_grouping`/
  `grouping_required`로 괄호 판정을 한 곳에 모으고, 값 방출 지점
  (scrutinee, propagate 입력, 값 전달, `result` 성공값, 파이프라인 head/step,
  arm guard, exit rewrite)에 적용.
- 테스트: `tests/compile.rs`에 레이아웃·그룹핑 회귀 4개 추가
  (`a_lowering_is_laid_out_from_the_line_it_replaces`,
  `a_nested_enum_declaration_is_laid_out_from_its_own_line`,
  `a_delivered_value_keeps_only_the_parentheses_that_group_it`,
  `a_postfix_step_parenthesizes_only_a_receiver_that_needs_it`). 기존 출력
  기대 55개를 새 형태로 갱신, `src/lib.rs`의 `ScrutineeTemp` doctest 갱신,
  `codegen::rope`의 조각 인덱스 단위 테스트 갱신.
- 검증: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `TTC_TSGO_ROOT=… TTC_REQUIRE_TSGO=1 cargo test` (native 39개 포함 전부 통과).
  typescript-go는 CI가 고정한 커밋(`c6b013f5`)을 클론해
  `go build -o built/local/tsgo ./cmd/tsgo` + `npx tsc -b _packages/native-preview`로
  빌드해 연동했다.

## 이슈 및 해결

### 이슈 1: 로프 길이 검증이 break를 셀 수 없다

- **증상**: `TargetFile::validate`는 조각 텍스트 길이의 합이 `Rope::len`과
  같은지 확인한다. break의 텍스트는 인쇄 시점에야 정해지므로 이 합이 맞지
  않는다.
- **원인**: `len`이 "출력 길이"와 "고정 텍스트 길이"를 겸하고 있었다.
- **해결**: break/스코프 조각은 길이 0으로 두고(`Piece::text`가 `""`),
  `len`은 고정 텍스트의 합이라는 의미로 좁혔다. 프린터의 버퍼 예약만
  `len + len/8`로 여유를 둔다. 검증은 그대로 유효하다 — 고정 조각의 합은
  여전히 정확히 일치해야 한다.

### 이슈 2: `emit_apply`의 head 괄호가 두 질문을 겸하고 있었다

- **증상**: `$tt_ap((xs), (f))`처럼 인자 위치인데도 괄호가 남았다.
- **원인**: head를 `(head)`로 한 번 감싸 두고 postfix 스텝이 그 앞에
  붙는 구조였다. 즉 "인자로 넘길 때"와 "멤버 접근의 수신자로 쓸 때"를 같은
  괄호가 담당했다.
- **해결**: head를 감싸지 않고 두고, 스텝 모드별로 필요한 쪽을 붙인다 —
  `Call`이면 `push_grouped`(콤마 규칙), `Postfix`면 `push_receiver`(primary
  규칙). `s |> .trim()`은 `s.trim()`, `(x + y) |> .toFixed(2)`는
  `(x + y).toFixed(2)`, `await p |> .then(g)`는 `(await p).then(g)`가 된다.

### 이슈 3: `result` 블록의 성공값이 원문 꼬리 공백을 물고 나왔다

- **증상**: `$tt_v1 = ({ kind: "Ok" as const, value: (Ok(x * y)\n  ) });`
- **원인**: 마지막 식의 span이 닫는 `}` 앞 개행·들여쓰기까지 포함하는데
  `.trim()` 없이 방출하고 있었다.
- **해결**: `emit_result_region`/`emit_result_region_continued` 양쪽에서
  `.trim()` 후 방출. 트림은 원본 오프셋을 함께 옮기므로 매핑은 그대로다.

### 이슈 4: 로컬에서 `engine_cache` 테스트 1개가 실패

- **증상**: `an_error_node_keeps_its_file_and_other_files_checkable`이 진단
  2개 대신 1개를 봤다. 작업 전 커밋(`76348f8`)에서도 동일하게 실패.
- **원인**: 이 테스트는 백엔드가 있는 환경을 전제한다 — 소진성 진단이 타입
  체커의 답에서 나온다. CI의 `check` 잡은 TypeScript 7을 설치하고
  `TTC_TSGO_API`를 걸어 `cargo test`를 돌리므로 그 환경에서는 통과한다.
  이번 변경과 무관하다.
- **해결**: 이 태스크의 검증도 CI와 같은 조건(빌드한 typescript-go를
  `TTC_TSGO_ROOT`로 연결)에서 돌렸고 전부 통과한다. 백엔드 없이도 tt 계층이
  단독으로 답해야 하는지는 별개의 질문이라 여기서 다루지 않는다.

### 이슈 5: 그룹핑 규칙의 안전망이 문자열 단언 하나뿐이었다

- **증상**: 커밋 후 자체 감사에서 변이 테스트를 돌렸다 — `needs_grouping`을
  항상 `false`로 만들어 괄호를 전부 제거해도 `tests/compile.rs`는 1개만
  실패하고, tsc·node로 **실제 실행**하는 `tests/integration.rs` 83개는 전부
  통과했다. 즉 정확성이 걸린 술어를 실행이 아니라 문자열 일치 하나가 지키고
  있었다.
- **원인**: 두 가지가 겹쳤다. ① 통합 테스트에 콤마식/비-primary 수신자
  케이스가 없었다. ② 처음 추가한 케이스는 값을 `(note(v), v + 10)`처럼 **원문에
  이미 괄호가 있는** 자리에서 썼다. 그런 자리는 컴파일러가 괄호를 더하지
  않아도 원문 괄호가 남으므로 규칙을 전혀 밟지 않는다. 규칙이 실제로 하중을
  받는 자리는 원문에 괄호가 없는 곳이다 — block arm의
  `return note(v), v + 10;`(exit rewrite가 대입으로 바꾼다)과
  `width + 0.5 |> .toFixed(1)`(head에 괄호가 없다).
- **해결**: 그 두 자리를 실행하는 통합 테스트
  `a_grouped_value_still_evaluates_to_what_the_arm_wrote`를 추가했다. 변이로
  확인한 결과: `grouping_required`를 무력화하면 `1 1 3.5 3`(대입이 왼쪽
  피연산자를 집는다), `push_receiver`를 무력화하면 `11 1 30.5 3`
  (`width + 0.5.toFixed(1)`)로 **둘 다 잡힌다**. 두 오방출 모두 문법적으로는
  멀쩡해서 출력 자가 검사(`verify_output`)는 통과한다 — 실행만이 잡는다.
  (스크루티니 쪽 오방출 `const $tt_m = a, b;`는 자가 검사가 잡는다.)

### 이슈 6: anchor 경계가 레이아웃 스코프까지 겸하고 있었다 (자체 감사)

- **증상**: 결정 2에서 `Rope::anchored`가 스코프까지 열게 했다. 두 경계가
  대부분 일치해서 동작은 했지만, "일치해야 한다"는 근거가 없었다. 빈 슬롯 이름
  하나를 감싸는 anchor도 의미 없는 스코프를 열고 있었고, break를 쓰는 emitter가
  스코프를 잊어도 아무도 알려주지 않았다.
- **원인**: 두 질문("이 글루는 어느 구문 것인가" / "이 글루의 블록 구조는 어디를
  기준으로 하는가")을 한 경계에 얹은 결합.
- **해결**: `anchored`는 스코프를 열지 않는다. break를 쓰는 emitter가 각자
  연다 — `emit_propagate`(try), `emit_value_decision`(match),
  `emit_result_region_continued`, `emit_apply_continued`. 잊었을 때를 위해
  불변식을 검증 계층에 넣었다: 스코프 밖의 break는
  `TargetError::BreakOutsideScope`다(anchor 균형 검사와 같은 자리). 변이로
  확인 — `emit_value_decision`의 스코프를 지우면
  `internal compiler error: invalid TypeScript target: Err(BreakOutsideScope)`로
  즉시 걸리고 compile 테스트 6개가 실패한다. 출력은 9개 파일(모든 구문 + TSX)에서
  **바이트 단위로 동일**하다 — 순수 리팩터링이다.

### 이슈 7: 레이아웃이 예시 테스트 2~3개로만 고정돼 있었다 (자체 감사)

- **증상**: `INDENT`를 4칸으로 바꾸는 변이에 3개만 실패했다. 기존 기대값 55개를
  스크립트로 리핏했기 때문에, 그 단언들은 레이아웃을 더 이상 붙잡지 않았다.
- **원인**: 규칙을 규칙으로 검사하지 않고 예시로 검사하고 있었다.
- **해결**: 속성 테스트
  `every_construct_lays_its_glue_out_from_the_line_it_replaces`. 10개 구문 ×
  4개 들여쓰기(탭 포함)에 대해 ① codegen이 **시작한** 줄은 전부 base + 온전한
  레벨만큼 들여쓰이고 ② 들여쓰기를 바꿔도 lower된 줄 수가 변하지 않는다는 것을
  검사한다. "codegen이 시작한 줄"은 텍스트 매칭이 아니라 emit map의 출처로
  판정하고(원문 바이트가 아닌 첫 글자로 시작하는 줄), 두 소스 랜드마크 사이로
  범위를 한정해 파일 스코프 헬퍼와 프렐류드를 배제한다. 인라인으로 lower되는
  구문(`if let`, let-else)은 줄을 만들지 않는데, 그것도 검사 대상이다 — 줄 수
  불변과 "코퍼스가 블록 구조를 실제로 밟고 있는가"를 함께 단언한다. 변이 확인:
  `line_indent`가 항상 빈 문자열을 돌려주게 하면(원래 버그 재현) 12개 실패.

### 이슈 8: 변이 검증 중 `git checkout`으로 커밋 안 한 작업을 날렸다

- **증상**: 이슈 6의 수정을 작업 중에 변이 테스트를 돌리고 `git checkout <파일>`로
  되돌렸더니, 같은 파일에 있던 미커밋 수정이 함께 사라졌다. 테스트가 계속
  통과해서 잠깐 눈치채지 못했다(결합된 구현에서는 그 변이가 보이지 않기 때문).
- **원인**: 변이 되돌리기와 작업 되돌리기가 같은 명령이었다.
- **해결**: 작업을 먼저 커밋하고 그 위에서 변이를 돌리는 순서로 바꿨다. 그래서
  이 태스크는 커밋이 여러 개다 — 속성 테스트, 그다음 분리.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (`TTC_TSGO_ROOT`로 typescript-go 연동, `TTC_REQUIRE_TSGO=1`)

## 결과

- `src/codegen/rope.rs` — 레이아웃 조각(Break/Scope)과 프린터의 해석,
  `push_break`/`scoped`/`indented`/`resolved_text`
- `src/codegen/core.rs` — 방출부 전체를 레이아웃 위로 이전, 괄호 판정 도입,
  enum 방출을 로프로
- `src/scanner.rs` — `has_top_level_comma`, `is_primary_expression`
- `src/lib.rs` — `ScrutineeTemp` doctest
- `tests/compile.rs` — 회귀 4개 추가, 출력 기대 갱신
- `docs/design/compiler-architecture.md`, `docs/ai/tt.md` — 레이아웃 규칙 기술

방출 코드는 이제 원문이 놓인 들여쓰기에서 시작하고, 괄호는 실제로 값을
묶어야 할 때만 남는다:

```ts
export function area(s: Shape): number {
  let $tt_v0;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "Circle": { const { radius } = $tt_m; $tt_v0 = Math.PI * radius * radius; break; }
      case "Rect": { const { w, h } = $tt_m; $tt_v0 = w * h; break; }
      case "Point": { $tt_v0 = 0; break; }
      default: { throw new Error("tt match: unexpected case " + JSON.stringify($tt_m)); }
    }
  }
  return $tt_v0;
}
```

후속 후보(이번 범위 밖): block arm의 도달 불가능한 `= undefined` 폴스루 제거
(flow CFG를 arm 본문까지 확장해야 한다), `do { … } while (false)` + 레이블
블록의 이중 중첩 단일화, 임시 이름(`$tt_y_$tt_v1`) 정리.
