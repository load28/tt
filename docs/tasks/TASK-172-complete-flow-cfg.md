# TASK-172: flow CFG 완성 — 모든 TypeScript 문 형태의 정확한 발산 판정

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: —

## 목적

`src/flow`의 CFG는 TASK-125에서 "최소 CFG"로 도입돼 `if`/`else`·바레 블록·
네 발산문만 모델링하고 나머지 문 형태(loop, `switch`, `try`, 레이블,
`do`-`while`)를 전부 fall-through로 근사했다. 근사 방향이 보수적이라 계약을
깨지는 않지만, **실제로 발산하는 `else` 블록을 컴파일러가 거부한다** — 사용자가
체감하는 오작동이다. CFG를 실제 제어 흐름 그래프로 완성해 발산 판정을
정확하게 만든다.

## 범위

- 포함: 문 스캐너의 재귀 하향 재작성, 레이블·`break`/`continue` 타깃 해석,
  loop(`while`/`do`-`while`/`for`/`for-in`/`of`/`for await`) · `switch` ·
  `try`/`catch`/`finally` · 레이블 문 모델링, 무한 루프 판정, ASI 문 경계,
  세 계층 테스트, 문서 갱신
- 제외: HIR body 연동(`Branch { condition: ExprId }` — Phase 5 잔여),
  tt 고유 구문(`match`/`if let`/`try`/`result`)의 발산 판정, 값 흐름·초기화
  분석, 도달 불가 코드 진단 표면화, `evaluation_ir`의 shadow CFG(별개 계층)

## 의사결정

### 결정 1: 분할 후 분류가 아니라 재귀 하향 문 스캐너로 재작성한다

- **상황**: 기존 `parse_statements`는 토큰 스트림을 top-level `;`와 블록 문의
  `}`로 **먼저 자른 뒤** 각 조각의 첫 단어로 분류했다. `switch`/loop/`try`를
  더하려면 조각 안에서 다시 구조를 찾아야 하는데, 그 조각 경계 자체가 이미
  틀려 있었다 — 예컨대 `if (c) for(;;) {} else …`는 `parse_embedded`가 첫
  top-level `;`까지 훑다가 `for(;;)`의 괄호 안 `;`를 건너뛰고 `else`까지
  삼켰다.
- **검토한 대안**: (A) 기존 분할기 위에 문 형태별 특수 케이스를 얹는다 —
  변경량은 작지만 경계 오류가 남고, 형태마다 다른 예외를 쌓게 되어
  CLAUDE.md 원칙 3(구조적·일반화된 해결)에 정면으로 어긋난다.
  (B) 문법을 그대로 따르는 재귀 하향 스캐너로 바꾼다 — 변경량은 크지만
  각 문 형태가 자기 모양을 스스로 인식하고, 모양이 안 맞으면 `Stmt::Other`로
  떨어져 보수성이 형태별 예외가 아니라 **하나의 원리**로 보장된다.
- **선택과 근거**: (B). `statement(at, end) -> (Stmt, next)` 하나가 모든 문
  형태의 진입점이 되고, 내장 문 위치(`if`의 then/else, loop body)도 같은
  함수를 재귀 호출한다. 그 결과 `parse_embedded`의 `;`-훑기 휴리스틱과
  "`}` 뒤에 `else`가 오면 자르지 않는다"는 특수 처리가 **둘 다 사라졌다**
  — `if`/`try`/`do`가 구조적으로 파싱되니 이어지는 키워드를 삼킬 여지가
  애초에 없다.

### 결정 2: `Terminator`에 `Switch`와 `Throw`를 더하지 않는다

- **상황**: 설계 문서(§9)는 `Terminator{Goto,Branch,Return,Throw,Match,End}`를
  적어 두었다. `switch`의 N-갈래 분기와 `throw`를 별도 종결자로 둘지
  정해야 했다.
- **검토한 대안**: (A) `Switch { targets: Vec<BlockId> }`를 추가한다 —
  `Terminator`가 `Copy`를 잃고(`Vec` 때문에) `diverges`의 값 매칭이 깨지며,
  얻는 정보는 없다. (B) `Throw`를 추가한다 — `try` 안의 `throw`가 함수를
  떠나는 게 아니라 handler로 간다는 걸 표현할 수 있다.
- **선택과 근거**: 둘 다 추가하지 않았다. `switch` dispatch는 **판별자를
  case에 순서대로 시험하는 것 그 자체**이므로 2-way `Branch` 사슬이 근사가
  아니라 정확한 표현이다. `throw`는 `Return`과 같이 함수를 떠나고,
  guarded block → handler 간선은 `Stmt::Try` lowering이 직접 그리므로
  (`Branch { then_bb: try_entry, else_bb: catch_entry }`) `Throw` 종결자가
  줄 정보가 이미 그래프에 있다. `Terminator`는 `Copy`인 5-variant 닫힌 합
  타입으로 유지됐다. 설계 문서 §9를 이 결정으로 갱신했다.

### 결정 3: `finally`는 정상 완료 경로에만 인라인한다

- **상황**: `finally`는 try/catch를 떠나는 **모든** 경로에서 실행된다.
  CFG로 정확히 표현하려면 경로별로 finally 블록을 복제해야 한다(rustc MIR의
  drop 복제, tsc binder와 같은 방식).
- **검토한 대안**: (A) 모든 이탈 경로(정상 완료·`return`·`break`·`continue`)에
  finally를 복제한다 — 정확하지만 abrupt exit을 finally를 거쳐 원래 목적지로
  다시 잇는 배관이 필요하다. (B) 정상 완료 경로에만 인라인한다.
- **선택과 근거**: (B). 근거는 abrupt exit에 대해 **복제가 답을 바꾸지
  않거나, 바꾸더라도 안전한 방향으로만 바꾼다**는 것이다. `return`/`throw`는
  finally가 정상 완료하든 발산하든 어차피 발산이고, 블록 안 레이블을 향하는
  `break`는 finally가 정상 완료하면 목적지가 같고 finally가 발산하면 *더*
  발산한다. 즉 생략은 "발산한다"를 놓칠 수는 있어도 없는 발산을 주장할 수는
  없다 — 계약(보수성)을 지킨다. 확인: `try { return 1; } finally { log(); }`
  발산 ✓, `try { log(); } finally { return 1; }` 발산 ✓,
  `try { log(); } finally { log(); }` 비발산 ✓ (`flow::tests`).

### 결정 4: handler 간선을 guarded block의 **진입점**에서 그린다

- **상황**: 예외는 guarded block의 어느 지점에서든 발생할 수 있다. 어디서
  handler로 가는 간선을 그릴지 정해야 했다.
- **검토한 대안**: 문마다 handler 간선을 그리면 블록 수가 문 수만큼 늘고,
  발산 판정에는 아무 차이가 없다(handler 도달 가능성은 동일).
- **선택과 근거**: 진입점 하나에서만 그린다. 그러면 "문장이 join에 도달한다
  ⟺ try 또는 catch 중 하나가 도달한다"가 되어 **`try`는 양쪽이 모두 발산할
  때만 발산한다**는 규칙이 그래프에서 정확히 나온다. 확인:
  `try { return 1; } catch (e) { log(e); }` 비발산 ✓,
  `try { return 1; } catch (e) { throw e; }` 발산 ✓.

### 결정 5: 무한 루프 판정은 소스 문자열이 아니라 토큰 위에서 한다

- **상황**: `while (true)`/`for (;;)`는 정상 이탈 간선이 없어야 정확한데,
  "조건이 항상 참인가"를 판정해야 했다. CLAUDE.md 원칙 3은 문자열 기반
  휴리스틱을 금지한다.
- **검토한 대안**: 조건 소스를 trim해 `"true"`와 비교 — 문자열 휴리스틱.
  상수 폴딩으로 `1`, `!0`, `"x"`까지 참으로 판정 — 이 계층에 상수 평가기가
  없고, 놓쳐도 보수적이라 이득이 작다.
- **선택과 근거**: **토큰 스트림 위의 구조적 진술**로 했다 — 중복 괄호를
  구조적으로 벗긴 뒤(`close(from) == to - 1`인 동안) 남은 토큰이 정확히
  하나이고 그것이 `true` 식별자일 때. C 스타일 `for`는 head의 top-level `;`
  두 개를 찾아 가운데 절이 비었는지로 판정하므로 `for (;;)`와
  `for (let i = 0;; i += 1)`이 같은 원리로 잡힌다. tsc의 binder가 쓰는
  기준(`TrueKeyword` 또는 조건 생략)과 동일하다. `for-in`/`of`는 조건 절이
  없고 0회 순회가 가능하므로 항상 이탈 간선을 갖는다.

### 결정 6: 제한된 자동 세미콜론(ASI) 규칙을 넣는다

- **상황**: 세미콜론 없는 스타일(prettier no-semi, standard)에서는 `else`
  블록 전체가 하나의 `Other` 문으로 뭉쳐 그 안의 모든 발산이 사라졌다.
  `log("x")\nreturn 0`이 거부됐다.
- **검토한 대안**: (A) 완전한 ECMA-262 ASI 구현 — 이 계층에 필요한 것보다
  훨씬 크고, 잘못 자르면 없는 발산을 주장할 위험이 있다. (B) 규칙을 넣지
  않는다 — 세미콜론 없는 코드베이스 전체가 오작동한다.
- **선택과 근거**: **모델링하는 문 형태 앞에서만** 자른다. 조건 셋을 모두
  만족해야 경계로 본다: ① 다음 토큰이 `STATEMENT_START_WORDS`(모델링하는 열
  가지 문 키워드) ② 앞 토큰이 식(expression)을 끝낼 수 있는 토큰
  (`ends_expression`) ③ 둘 사이에 줄바꿈. 이 셋을 다 만족하는데 실제로는
  한 문인 경우는 없고(그 키워드들은 식을 이어갈 수 없다), 반대로 놓치면
  `Other` 하나가 길어질 뿐이라 보수적이다. `.`/`?.` 뒤는 ②에서 자동
  배제되므로 `x.return`/`iter.return()`은 잘리지 않고, 객체 리터럴 안의
  `{ return: 1 }`은 depth > 0이라 검사 대상이 아니다. `break`/`continue`의
  레이블도 같은 이유로 restricted production을 지킨다(줄바꿈이 있으면
  레이블로 읽지 않는다).

### 결정 7: `break`/`continue`를 "블록을 떠난다"에서 "타깃을 해석한다"로 바꾼다

- **상황**: 기존 모델은 `break`/`continue`를 무조건 `Terminator::Jump`
  (분석 대상 블록을 떠남)로 봤다. loop/`switch`를 모델링하지 않을 때는
  우연히 안전했지만(그 안으로 내려가지 않았으므로), 모델링하는 순간
  `while (true) { break; }`가 발산으로 잘못 판정된다 — **없는 발산을
  주장하는 불건전성**.
- **선택과 근거**: lowering 중에 `Scope` 스택을 유지한다. 각 scope는
  레이블·종류(`Iteration`/`Switch`/`Labeled`)·`break` 착지점·`continue`
  착지점을 갖는다. 해석 규칙은 언어 규칙 그대로다 — 레이블 없는 `break`는
  가장 안쪽 loop 또는 switch, 레이블 없는 `continue`는 가장 안쪽 loop,
  레이블 있는 것은 그 레이블을 단 scope. 분석 대상 블록 **안에서** 타깃을
  찾지 못하면 그때만 `Terminator::Jump`이며, 이것이 기존 "네 키워드는
  블록을 떠난다"의 정확한 일반화다. 확인: `while (true) { break; }` 비발산 ✓,
  `outer: while (true) { while (true) { break outer; } } return 0;` 발산 ✓.

## 작업 내역

- 2026-08-23: 착수. 현행 CFG의 결함을 실제 컴파일로 확인했다. 진짜 발산하는
  `else` 블록 다섯 형태(`switch`+`default`, `while (true)`,
  `try`/`catch`, 레이블 `break`, `do`/`while`)를 `.tt` 파일로 만들어
  `ttc`에 넣었고, 다섯 개 모두 `let-else: every path through the else block
  must diverge` 오류로 거부되는 것을 확인했다.
- 2026-08-23: `src/flow/mod.rs`의 문 모델과 lowering을 재작성했다.
  `Stmt`에 `Break(Option<&str>)`/`Continue(Option<&str>)`/`Labeled`/
  `Loop { kind, body }`/`Switch(Vec<Clause>)`/`Try { block, catch, finally }`를
  더하고, `LoopKind { test_first, exits }`로 모든 iteration 문을 두 축으로
  일반화했다(`do`-`while`만 `test_first: false`, 조건이 없거나 `true`일 때만
  `exits: false`).
- 2026-08-23: `Builder`에 `set`(예약 블록 채우기)을 더해 loop의 back edge를
  표현했다 — 종료 시험 블록을 body보다 먼저 예약하고 body의 back edge가
  그것을 가리킨 뒤 채운다. `seq`를 재귀에서 뒤→앞 반복으로 바꿔 문 수에
  비례하던 스택 깊이를 없앴다.
- 2026-08-23: `Scope`/`ScopeKind`와 `break_target`/`continue_target`으로
  레이블 해석을 구현했다(결정 7).
- 2026-08-23: `parse_statements`/`classify`/`parse_if`/`parse_embedded`/
  `find_close`를 지우고 `Scanner`로 대체했다 — `statement`,
  `if_statement`, `while_statement`, `do_statement`, `for_statement`,
  `switch_statement`, `try_statement`, `clauses`, `case_colon`,
  `statement_end`, `asi_boundary`, `always_true`, `head_separators`.
- 2026-08-23: 회귀 방지 행렬을 `.tt` 파일 22개로 만들어 돌렸다 — 거부돼야
  하는 12개(`switch` without `default`, `case`에서 `break`, 이탈 가능한
  조건, `for-of`, handler가 정상 완료, `finally`가 정상 완료, 레이블 블록
  탈출 후 끝, `while (true) { break; }` 등)와 통과해야 하는 10개. 22개 모두
  기대대로 나왔다.
- 2026-08-23: 렉서 토큰 경계 엣지 케이스를 확인했다 — 템플릿 리터럴·정규식·
  JSX는 단일 토큰이라 그 안의 `case`/`return`/`;`/`{`가 스캐너에 보이지
  않고, `case "{"`·삼항이 든 `case` 라벨·객체 리터럴의 `default:` 키가 모두
  정확히 처리된다.
- 2026-08-23: 세 계층 테스트를 더했다 — `src/flow` 단위 테스트 6개
  (loop/`break` 타깃/레이블 블록/`switch`/`try`/ASI),
  `tests/compile.rs`에 컴파일러 계층 2개(발산 11형태 통과 + 정상 이탈
  11형태 거부), `tests/integration.rs`에 런타임 1개
  (`switch`·`try`/`catch`·`finally`·레이블 `break`·무한 루프 다섯 형태를
  `tsc --strict`로 타입체크하고 `node`로 실행해 값까지 확인).
- 2026-08-23: `src/hir/mod.rs`의 `else_diverges_hint`를 `else_diverges`로
  바꾸고 주석을 고쳤다 — TASK-125 이후로 이미 CFG 답이었는데 "파서의 구문적
  힌트(마지막 문이 무슨 키워드로 시작하는지)"라고 적혀 있어 구현과 어긋났다.
  `src/ast.rs`의 `diverges` 주석도 같은 이유로 고쳤다.
- 2026-08-23: `docs/design/compiler-core.md` §9와 Phase 5 잔여 항목,
  `docs/ai/tt.md`의 발산 규칙 문단을 갱신했다.
- 2026-08-23: 검증 게이트를 돌렸다. `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test` 전부 통과
  (739 테스트).

## 이슈 및 해결

### 이슈 1: `engine_cache` 테스트가 이 컨테이너에서 실패 — 변경과 무관

- **증상**: `cargo test`에서
  `engine_cache::an_error_node_keeps_its_file_and_other_files_checkable`가
  `assertion failed: left: 1, right: 2` (진단 2개를 기대했는데 1개)로
  실패했다.
- **원인**: `git stash`로 변경을 걷어내고 다시 돌려도 같게 실패해 이번
  변경과 무관함을 먼저 확인했다. 이어서 문제 파일 하나만 두고
  `ttc --check-types .`를 돌리니 여전히 진단이 없었고, 로그가
  `no TypeScript compiler found`를 알렸다. 기대된 진단은
  `match (E.A(1))`의 소진성인데, 판별자가 생성자 호출이라 그것이 enum `E`임을
  아는 데 typed facts가 필요하다. 백엔드가 없으면 TASK-124의 설계대로
  typed facts가 unknown으로 강등되고 소진성 검사가 성립하지 않는다.
  즉 **코드 결함이 아니라 이 컨테이너에 TypeScript 7 백엔드가 없어서**였다
  (`.github/workflows/ci.yml`은 `cargo test` 전에 typescript@7을 설치하고
  `TTC_TSGO_API`를 걸어 둔다).
- **해결**: CI의 `native` 잡과 같은 방식으로 백엔드를 직접 마련했다 —
  `microsoft/typescript-go`를 CI가 고정한 커밋
  `c6b013f5706d58582f566df778cc0df2683b58f5`로 얕은 클론하고,
  `go build -o built/local/tsgo ./cmd/tsgo` + `npm ci` +
  `npx tsc -b _packages/native-preview`로 빌드한 뒤 `TTC_TSGO_ROOT`로
  가리켰다. 그 상태에서 전체 스위트가 739개 전부 통과했고,
  CI의 skip 방지 게이트(`TTC_REQUIRE_TSGO=1 cargo test --test native`)도
  39개 전부 skip 없이 통과했다. 남은 부채 없음.

### 이슈 2: 기존 테스트 하나가 옛 근사 동작을 고정하고 있었다

- **증상**: `flow::tests::loops_and_try_are_conservatively_fall_through`가
  `assertion failed: !check("for (;;) { return 1; }")`로 실패했다.
- **원인**: 그 테스트는 TASK-125가 의도적으로 남긴 근사(loop·`switch`·`try`는
  fall-through)를 **계약처럼 고정**한 것이다. 이번 태스크가 바로 그 근사를
  없앤다.
- **해결**: 테스트를 지우지 않고 그 자리에 정밀한 기대치로 교체했다 —
  `a_loop_diverges_only_when_it_has_no_normal_exit`,
  `a_break_leaves_the_loop_it_names_not_the_block`,
  `a_labeled_block_is_breakable_but_not_continuable`,
  `a_switch_diverges_when_it_has_a_default_and_no_clause_falls_out`,
  `a_try_diverges_when_every_half_that_can_complete_does_not`,
  `statements_read_the_same_without_semicolons`. 근사가 사라진 자리마다
  양방향(발산/비발산) 기대치를 남겨 불건전한 방향의 회귀를 막았다.

### 이슈 3: clippy `redundant_closure`

- **증상**: `cargo clippy --all-targets -- -D warnings`가
  `src/flow/mod.rs:308,313`에서 `redundant closure`로 실패했다.
- **원인**: `break`/`continue` 타깃 해석에서
  `.map_or(Terminator::Jump, |target| Terminator::Goto(target))`로 썼는데,
  튜플 variant 생성자를 그대로 넘길 수 있다.
- **해결**: `.map_or(Terminator::Jump, Terminator::Goto)`로 바꿨다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 739개 통과 (직접 빌드한 typescript-go 백엔드 연동 상태)
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native` — 39개, skip 0

## 결과

발산 판정이 근사에서 실제 그래프 답으로 바뀌었다. 진짜 발산하는데 거부되던
`else` 블록 형태(`switch`+`default`, 정상 이탈 없는 loop, `try`/`catch`,
발산하는 `finally`, `do`/`while`, 레이블 `break`, 세미콜론 없는 문)가 모두
통과하고, 발산하지 않는 형태는 전부 그대로 거부된다.

변경 파일:

| 파일 | 변경 |
|------|------|
| `src/flow/mod.rs` | 문 모델·스캐너·lowering 재작성, 단위 테스트 교체 |
| `src/hir/mod.rs`, `src/hir/lower.rs` | `else_diverges_hint` → `else_diverges`, 주석 정정 |
| `src/ast.rs` | `diverges` 주석 정정 |
| `tests/compile.rs` | 컴파일러 계층 테스트 2개 |
| `tests/integration.rs` | 런타임 계층 테스트 1개 |
| `docs/design/compiler-core.md` | §9와 Phase 5 잔여 갱신 |
| `docs/ai/tt.md` | 발산 규칙 문단 갱신 |

후속으로 등록할 만한 것(이번 범위 밖):

- **tt 고유 구문의 발산** — `else` 블록 안의 `match`/`if let`/`try`/`result`는
  여전히 fall-through다. 이 계층은 구문 파싱 이전의 토큰 스트림 위에서 돌아
  순환 의존 없이 판정할 수 없다. `HIR::LetElseStmt::else_body`(이미 lowered
  body를 들고 있다) 위의 flow 패스가 제자리이며, 이는 설계 문서가 말하는
  Phase 5 잔여(`Branch { condition: ExprId }`)와 같은 작업이다.
- **도달 불가 코드 진단** — 그래프는 이미 도달 불가 블록을 알지만
  (`return` 뒤의 문) 진단으로 표면화하지 않는다.
