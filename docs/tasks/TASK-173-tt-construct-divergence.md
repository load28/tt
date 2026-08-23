# TASK-173: tt 구문의 발산 판정 — flow의 마지막 근사 제거

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**:, `e1fe434``32ea4d2`

## 목적

TASK-172가 flow CFG의 TypeScript 문 커버리지를 완성했지만, tt 자신의 구문
(`match`/`if let`/`try`/`result`/중첩 let-else)은 여전히 fall-through로
남겨 두었다 — "이 계층은 구문 파싱 이전의 토큰 스트림 위에서 돌기 때문"이라는
이유였다. 그 남은 근사를 없앤다.

## 범위

- 포함: tt 구문별 발산 가능성의 **배치 사실 기반 확정**, `if let`의 발산
  판정 구현, 파서와 flow 사이의 경계 정보 전달, 세 계층 테스트, 문서 정정
- 제외: `Branch { condition: ExprId }`(아래 결정 4), 문 수준 도달 불가 코드
  진단(아래 결정 5)

## 의사결정

### 결정 1: 어떤 tt 구문이 발산할 수 있는지를 배치(Place) 규칙에서 확정한다

- **상황**: "tt 구문은 fall-through"라고 뭉뚱그려 두었지만, 어떤 구문이
  실제로 블록의 발산을 나를 수 있는지 먼저 확정해야 무엇을 구현할지 정해진다.
- **검토한 대안**: 구문마다 개별적으로 "이건 발산할 수 있나?"를 판단하면
  기준이 구문 수만큼 생기고, 새 구문이 추가될 때 또 판단해야 한다.
- **선택과 근거**: `sema`가 이미 소유한 **`Place` 규칙 하나**로 답했다.
  거기 적힌 그대로다 — "An `if let` body and a let-else `else` block are
  **inline**: their statements run where the statement itself stands. A
  match arm body, a `result` block's statements, and every isolated value
  region reset to `Place::ValueRegion` — an exit written there belongs to
  the construct value, never the user's function." 이 한 규칙에서 결론이
  기계적으로 나온다:

  | 구문 | 배치 | 발산 가능? | 근거 |
  |------|------|-----------|------|
  | `if let` body/else | inline | **가능** | exit이 바깥 함수를 떠난다 |
  | match arm body | isolated | 불가 | exit이 구문 값에 속한다 |
  | `result` 블록 | isolated | 불가 | 같음 |
  | `try` 문 | inline이지만 조건부 | 불가 | 성공 경로가 항상 있다 |
  | 중첩 let-else | inline이지만 조건부 | 불가 | else 발산 후 계속 진행 |

  즉 **구현할 것은 `if let` 하나**이고, 나머지 넷에 대한 "발산하지 않는다"는
  보수적 근사가 아니라 **정답**이다. 문서에서 이 넷을 "conservative
  fall-through"라 부르던 서술은 부정확했으므로 정정했다.

### 결정 2: `if let`의 경계는 flow가 다시 판정하지 않고 파서에게 받는다

- **상황**: flow가 `if let`을 문으로 인식하려면 패턴이 어디서 끝나고 어느
  `{`가 then-block을 여는지 알아야 한다. 그런데 그 판정은
  `parser::iflets`가 이미 하고 있다.
- **검토한 대안**: (A) flow의 토큰 스캐너 안에서 `if let` head를 다시
  파싱한다 — 자족적이지만 **한 규칙에 구현이 둘** 생긴다. 파서 쪽이 바뀌면
  조용히 어긋나고, 어긋난 순간 flow가 잘못된 블록을 then-body로 읽어
  없는 발산을 주장할 수 있다. CLAUDE.md 원칙 3의 "구조적 해결" 기준에
  정면으로 어긋난다. (B) `Program`(AST)을 통째로 낮춘다 — tt 세그먼트가
  문 중간에서 끊기므로(`log(match (e) {...});`는 Verbatim+Match+Verbatim)
  세그먼트별로 스캔하면 문 경계가 깨진다. 세그먼트를 다시 이어 붙이는
  배관이 필요하다.
- **선택과 근거**: (C) — 두 계층이 **각자 아는 것만** 답하게 이었다. 파서가
  `if let`의 `if` 바이트 오프셋 → head가 끝나는 바이트 맵(`IfLetHeads`)을
  넘기고, 스캐너는 "이 토큰에서 tt 문이 시작하는가, head는 어디서
  끝나는가"만 묻는다. 그 뒤로는 `{`를 찾아 then-block을 잡고 `else`를 잇는
  **`if`와 완전히 같은 코드 경로**다 — 제어 흐름도 같기 때문이다. 스캐너는
  여전히 토큰 스트림 전체를 보므로 (B)의 문 경계 문제가 생기지 않는다.

### 결정 3: 발산 판정 진입점을 `block_diverges`에서 `program_diverges`로 바꾼다

- **상황**: 파서가 `else_body`를 만들기 **전에** `block_diverges(src,
  tokens)`를 호출하고 있었다. tt 구문을 보려면 파싱된 `Program`이 필요하다.
- **선택과 근거**: `parser/lets.rs`에서 `else_body`를 먼저 만들고
  `program_diverges(src, tokens, &else_body)`를 호출하도록 순서를 바꿨다.
  파서는 여전히 infallible하고(판정은 bool, 보고는 sema), 순환 의존도 없다
  — flow는 `ast`에만 의존하고 `parser`에 의존하지 않는다. 소비자가 사라진
  `block_diverges`/`lower_block`은 남기지 않고 지웠다.

### 결정 4: `Branch { condition: ExprId }`는 구현하지 않는다

- **상황**: 설계 문서 §13이 Phase 5 잔여로 적어 둔 항목이다.
- **검토한 대안**: 지금 조건을 `ExprId`로 들도록 `Terminator`를 넓힐 수 있다.
- **선택과 근거**: **구현하지 않는 것이 원칙에 맞다.** 두 가지 이유다.
  ① 발산 판정에는 조건 식별이 필요 없다 — 모든 분기를 도달 가능으로 보는
  쪽이 보수적으로 옳고, 조건을 평가하면 오히려 없는 발산을 주장할 위험이
  생긴다. ② 조건을 들 유일한 이유인 분기별 초기화는 설계 문서가 이미
  **"소비 구문이 없어 보류"**라고 적어 둔 상태다 — `val let`은 재대입이
  설계상 허용이라 지연 초기화가 이미 성립하고, 불변 지연 초기화
  (`val const x;` + 분기별 1회 대입)는 **새 언어 표면 제안**이 선행돼야
  한다. 소비자 없이 IR만 넓히는 것은 구조 개선이 아니라 부채다. 설계
  문서의 해당 항목을 이 판단으로 갱신했다.

### 결정 5: 문 수준 도달 불가 코드 진단은 내지 않는다

- **상황**: 그래프는 도달 불가 블록을 이미 안다(`return` 뒤의 문). 진단으로
  표면화할지 정해야 했다. `Severity::Warning` 채널도 이미 있다.
- **검토한 대안**: let-else의 `else` 블록에 한해 경고를 낸다 — 그래프가
  이미 있는 유일한 영역이라 구현이 싸다.
- **선택과 근거**: **내지 않는다.** ① 원칙 2가 통과 영역의 TS 판정을 tsc에
  맡긴다. 문 수준 도달 불가는 tsc가 `allowUnreachableCode`로 이미 소유한
  진단이다. ② tt가 파일 전체에 대해 내려면 모든 함수 본문에 CFG를 돌려야
  하는데, 그것은 tsc CFG의 재구현이다. ③ `else` 블록에만 내면 **같은 코드가
  놓인 위치에 따라 다르게 취급**돼 원칙 3("구조적으로 같은 모든 입력에
  적용되는 하나의 원리")에 어긋난다. 한편 tt가 실제로 소유하는 도달 불가
  — 패턴 arm의 usefulness — 는 `analysis/usefulness.rs`에 **이미 구현되어
  있다**(`Severity::Warning`의 주석이 가리키는 그것). 즉 이 항목은 tt 몫과
  tsc 몫이 이미 각자 제자리에 있다.

## 작업 내역

- 2026-08-23: 어떤 tt 구문이 실제로 발산할 수 있는지 `.tt` 프로브로 먼저
  실증했다. `if let` 양쪽 발산(u3)이 거부되는 것을 확인했고, match arm의
  `return`은 arm body가 표현식이라 애초에 파싱되지 않음을 확인했다.
- 2026-08-23: **불건전성 회귀가 없는지 먼저 검증했다.** TASK-172의 재작성이
  기존 `parse_statements`의 "`}` 뒤에 `else`가 오면 자르지 않는다" 규칙을
  없앴으므로, `if let ... { 발산 안 함 } else { return 1; }`의 `else` 블록이
  독립 블록으로 읽혀 발산으로 오판정될 수 있었다. 프로브 6종(s1–s6)으로
  전부 안전함을 확인했다 — `else`는 `NON_LABEL_WORDS`의 키워드라
  `Stmt::Other`가 되고 블록으로 내려가지 않는다.
- 2026-08-23: `src/flow/mod.rs`에 `IfLetHeads`(파서→flow 경계 정보),
  `program_diverges`, `collect_if_let_heads`/`collect_if_let`,
  `Scanner::if_let_head`/`if_let_statement`를 구현했다. `Scanner`가
  `if_lets: &IfLetHeads`를 들고, `word(at) == "if"`일 때 맵을 먼저 조회한다.
- 2026-08-23: `src/parser/lets.rs`에서 `else_body`를 먼저 만들고
  `program_diverges`를 호출하도록 바꿨다. 소비자가 사라진
  `block_diverges`/`lower_block`을 삭제했다.
- 2026-08-23: `flow`의 테스트 헬퍼 `check`를 `program_diverges`를 쓰도록
  바꿔, 단위 테스트가 컴파일러와 **같은 경로**를 검증하게 했다.
- 2026-08-23: 양방향 프로브 10종(i1–i5 발산, j1–j5 비발산)으로 `if let`
  체인·중첩·한쪽만 발산·else 없음을 확인했다. 10종 모두 기대대로 나왔다.
- 2026-08-23: 세 계층 테스트를 더했다 — `src/flow` 단위 2개
  (`an_if_let_diverges_when_both_of_its_inline_halves_do`,
  `an_isolated_value_region_cannot_carry_the_blocks_divergence`),
  `tests/compile.rs` 2개, `tests/integration.rs` 런타임 1개(`if let` 체인과
  중첩을 `tsc --strict`로 타입체크하고 `node`로 값까지 확인).
- 2026-08-23: `docs/ai/tt.md`와 `docs/design/compiler-core.md` §9·§13을
  정정했다 — 넷을 "conservative fall-through"라 부르던 서술이 부정확했다.
- 2026-08-23: 검증 게이트 3종을 통과했다.

## 이슈 및 해결

### 이슈 1: 진입점 교체 후 `block_diverges`가 죽은 코드가 됐다

- **증상**: `cargo build`가 `function block_diverges is never used`,
  `function lower_block is never used` 경고를 냈다. clippy `-D warnings`가
  실패한다.
- **원인**: 유일한 소비자였던 `parser/lets.rs`가 `program_diverges`로
  옮겨갔고, 남은 사용처는 테스트 헬퍼뿐이었다.
- **해결**: `#[cfg(test)]`로 살려 두지 않고 **삭제**했다. 테스트 헬퍼를
  `program_diverges`로 바꾸는 편이 오히려 낫다 — 단위 테스트가 컴파일러와
  같은 경로를 타게 되고, 그 덕분에 `if let` 테스트를 `flow` 단위 테스트에서
  바로 쓸 수 있게 됐다.

## 회귀 검증 (TASK-172·173 합산)

두 태스크가 `src/flow`를 사실상 재작성했으므로, 기존 동작에 영향이 있는지
별도로 감사했다.

### 1. 소비자 표면 — 변경이 닿는 범위

`flow` 밖으로 나가는 것은 `program_diverges`(`parser/lets.rs`)와
`in_function_body`(`parser/mod.rs`) 둘뿐이다.

- `in_function_body`와 그 헬퍼(`function_body_brace`/`paren_heads_function`/
  `find_open`)는 **한 줄도 바뀌지 않았다**
  (`git diff e1adda8 -- src/flow/mod.rs`로 확인).
- `brace_opens_statement`는 `pub(crate)`지만 실제 소비자는 flow 내부 하나뿐.
- `parse_tokens`는 `&self`(불변 참조)라 `parser/lets.rs`에서 호출 순서를
  바꾼 것이 부작용을 낳을 수 없다.

### 2. 통과 계약 (절대 원칙 1) — 실제 TypeScript 1194개 차등 비교

`microsoft/typescript-go` 체크아웃의 `.ts`/`.tsx` 1194개(테스트 픽스처와
`node_modules` 포함 — 손으로 만든 예제가 아닌 실제 코드)를 변경 전
커밋(`e1adda8`)의 ttc와 현재 ttc로 각각 컴파일해 **출력과 진단 로그를 바이트
단위로 비교**했다.

```
1194 SAME / 0 DIVERGED
```

일반 TypeScript에 대한 동작 변화는 0이다.

### 3. 발산 판정 차등 — 2162개 조합 케이스

`else` 블록 본문 조각 45종(네 발산문, if/else 체인, 모든 loop 형태, switch
5종, try 5종, 레이블, 내부 함수, 객체 리터럴, 세미콜론 없는 변형)을 단독과
2개 조합으로 2162개 생성해 두 바이너리의 판정을 비교했다.

| 전이 | 건수 | 뜻 |
|------|------|-----|
| N→Y | 1022 | 이전에 **잘못 거부**하던 것을 받아들임 (개선) |
| Y→Y | 632 | 동일 |
| N→N | 508 | 동일 |
| **Y→N** | **0** | **이전에 통과하던 것이 거부되는 회귀 — 없음** |

변화는 전부 한 방향이다.

### 4. 불건전성 — tsc를 정답지로 대조

"더 받아들인다"가 안전한지는 차등만으로 알 수 없으므로, **TypeScript
자신의 제어 흐름 분석**을 정답지로 삼았다. 각 본문을
`function probe(): number { <본문> }`에 넣고 `tsc --strict --noEmit`을
돌리면, 발산하지 않는 본문에만 TS2355/TS2366/TS7030이 나온다. 구문 오류
(TS1xxx)가 난 함수는 tsc의 CFG 판정이 무의미하므로 제외했다.

```
대조한 케이스                        : 1980
불건전 (tt=발산, tsc=발산 아님)      : 0
놓침   (tt=발산 아님, tsc=발산)      : 3
```

**tt가 "발산한다"고 답한 것 중 tsc가 반박한 것은 하나도 없다.**

### 5. 놓침 3건 — 모두 안전한 방향이고, 둘은 의도된 설계

1. `switch (k) { case "a": break; default: return 1; }` 뒤에
   `switch (k) { case "a": return 1; }` — tsc는 첫 switch의 `case "a"` 경로에서
   `k`가 `"a"`로 **타입 narrowing**됨을 알아 두 번째 switch가 소진적이라고
   판정한다. flow는 조건·판별자를 평가하지 않으므로(설계 결정) 범위 밖이다.
2·3. 세미콜론 없는 코드에서 `{ … }` 블록 문과 라벨 문이 앞 문에 붙는 경우.
   **`{` 앞에서 자르지 않는 것은 의도적으로 옳다** — 자르면 Allman 스타일
   `function g()\n{ return 1; }`에서 내부 함수의 `return`이 블록으로 새어나와
   **불건전**해진다. 프로브로 확인했고
   (`a_brace_on_its_own_line_does_not_start_a_statement`), 회귀 테스트로
   고정했다.

### 6. 그 밖

- **TSX/JSX**: `JsxRaw`가 단일 토큰이라 JSX 안의 `"return 1;"` 같은 텍스트가
  문으로 오독되지 않는다. `.ttx` 양방향 프로브로 확인.
- **성능**: 80KB 이상 대용량 파일 10개 기준 base 0.336s / new 0.318s — 차이 없음.
- **전체 스위트**: 746개 통과.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 746개 통과 (직접 빌드한 typescript-go 백엔드 연동 상태)

## 결과

flow CFG에서 **tt 구문에 대한 근사가 사라졌다**. 남은 근사는 조건의 상수성
판정 하나뿐이고(리터럴 `true`와 생략된 조건만 "실패할 수 없는 시험"으로
본다 — tsc binder와 같은 기준), 그 방향의 오차는 "발산하지 않는다"로만
기울어 계약상 안전하다.

실질적으로 달라진 것: 양쪽이 모두 발산하는 `if let`이 이제 let-else의
`else` 블록을 발산시킨다(체인·중첩 포함). 나머지 tt 구문에 대한 답은 그대로
"발산하지 않는다"이지만, 그것이 근사가 아니라 배치 규칙에서 따라 나오는
정답임이 확정되고 문서에 반영됐다.

변경 파일:

| 파일 | 변경 |
|------|------|
| `src/flow/mod.rs` | `IfLetHeads`·`program_diverges`·`if_let_statement` 추가, `block_diverges`/`lower_block` 삭제, 단위 테스트 2개 |
| `src/parser/lets.rs` | `else_body`를 먼저 파싱하고 `program_diverges` 호출 |
| `tests/compile.rs` | 컴파일러 계층 테스트 2개 |
| `tests/integration.rs` | 런타임 계층 테스트 1개 |
| `docs/design/compiler-core.md` | §9 정정, §13 Phase 5 잔여를 결정 4로 정산 |
| `docs/ai/tt.md` | tt 구문 발산 규칙 정정 |
| `docs/tasks/TASK-172-*.md` | 부정확했던 후속 서술 정정 |

설계 문서의 Phase 5는 이 태스크로 **정산 완료**다 — 남은
`Branch { condition: ExprId }`는 결정 4대로 소비 구문이 생길 때까지 보류이며,
그 소비 구문(불변 지연 초기화)은 새 언어 표면 제안이 선행돼야 한다.
