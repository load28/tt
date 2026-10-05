# 컴파일러 아키텍처 — 단계 분리 파이프라인

이 문서는 ttc 내부 구조의 규범 설명이다. TASK-010에서 단일 패스 구조를
swc 스타일의 단계 분리 파이프라인으로 재구성했고, TASK-021에서 파서
프런트엔드에 렉서(토큰화) 단계를 도입해 파서가 바이트가 아닌 토큰 스트림
위에서 동작하게 했다. 모듈 배치가 이 문서와
어긋나면 버그로 취급한다. (이전 구조의 역사적 배경은
[`rust-rewrite.md`](./rust-rewrite.md) 참조 — 그 문서의 모듈 배치 설명은
이 문서가 대체한다.)

## 왜 단계를 분리했나

초기 구현은 한 번의 바이트 스캔 루프 안에서 파싱·의미 검사·코드 방출을
동시에 수행했다. 구현은 작았지만, 기능을 추가할 때마다 세 관심사를 한
함수에서 같이 건드려야 했고, 에러 전파(`Result`)가 파싱·방출 전체에
퍼져 있었다. swc가 `swc_ecma_ast` / `swc_ecma_parser` /
`swc_ecma_transforms` / `swc_ecma_codegen`으로 단계를 나누듯, ttc도
단계마다 독립 모듈을 두고 단계 간 계약을 타입드 AST로 명시한다.

## 파이프라인

```
소스 텍스트
   │  lexer::lex_with_kind   — 유의 토큰 스트림 (트리비아 제외, 스팬 보존)
   ▼
Vec<Token>
   │  parser::parse          — 무오류(infallible) 구조 파싱
   ▼
ast::Program                 — 단계 간 계약
   │  sema::check            — AST 위의 모든 tt 수준 에러 (Result<(), TtError>)
   │  val::check             — 토큰 스트림 위의 `val` 바인딩 분석 (같은 에러 타입)
   ▼
ast::Program (검증됨)
   │  codegen::emit          — 무오류 방출
   ▼
TypeScript 텍스트
   │  verify::verify_output  — swc 파싱 자가 검사 (--no-verify로 생략)
   ▼
최종 출력
```

### 1. `ast` — 단계 간 계약

파싱된 파일은 `Program` = 소스 순서의 `Segment` 목록이다:

- `Verbatim(Span)` — tt 구문이 아닌 모든 것. 원본 바이트 범위 그대로.
- `Variant(VariantDecl)` / `Match(MatchExpr)` / `Try(TryStmt)` /
  `LetElse(LetElseStmt)` — 완전하게 파싱된 tt 구문.
- `TtImport(Span)` — 정적 import/re-export의 상대 경로 `.tt` 지정자 문자열
  (따옴표 포함). 문장의 나머지는 verbatim으로 남고, codegen이
  `ImportRewrite` 모드에 따라 확장자를 재작성한다.
- `Template(Template)` — 템플릿 리터럴. 보간(`${ }`)마다 재귀 `Program`.

match의 scrutinee와 arm body도 재귀 `Program`이라 트리가 균일하다. 모든
Span/오프셋은 원본 소스의 절대 바이트 위치다 — 이것이 의미 에러를
`파일:행:열`로 되돌리는 연결 고리다.

### 2. `lexer` — 토큰화

As swc lexes TypeScript before parsing it, the source first becomes a
**stream of significant tokens**: white space and comments are trivia and
produce no tokens (verbatim emission copies the original bytes, so they need
no representation), and strings, templates, and regular expressions are
atomic tokens. Templates are lexed hierarchically: each `${ }`
interpolation's token stream rides inside its template token. Only the
operators the parser must consume as units are fused (`=>`, `||`, `?.`,
`??`, `|>`); every other significant byte is a one-byte `Punct`. The byte
primitives — string, regular-expression, and trivia scanning, and the
ECMA-262 `LineTerminator` set (LF, CR, U+2028, U+2029) — live in
`scanner.rs`. The questions codegen asks of text it emits
(`contains_await`, `is_primary_expression`, `is_member_receiver`,
`has_top_level_comma`) are
answered over tokens in `lexer/queries.rs`.

**Token facts (TASK-491).** Statement boundaries are modeled once, in the
lexer layer. `lexer/facts.rs` is a push-down recognizer for TypeScript's
statement and expression skeleton — statement lists, statements,
expressions, bracketed groups, object, class, and interface bodies, and a
small type grammar entered after annotations, `as`/`satisfies`, type
arguments, heritage clauses, and type aliases — that the lexer drives one
token at a time. It records `TokenFacts` on every token: a line terminator
before it, whether it completes an operand, whether an automatic semicolon
precedes it (§12.10.1, restricted productions included), whether it starts
a statement, whether it is a label or a member name, on a `{`, whether
it opens a function body (a generator's or a constructor's), and, on a `<`
and its `>`, whether they bracket type arguments or type parameters
(TASK-495). `Token::opens_bracket` and `Token::closes_bracket` read that
last fact, and every bracket walk in the parser, flow, `val`, completion,
and the lexer's token queries balances brackets through them, so a `,`
inside `f<A, B>(x)` is never taken for a top-level separator. Where
TypeScript's parser looks ahead to classify a token, the machine applies
the same rule by peeking at the bytes after it: a `(` in a type opens a
function type's parameters only when `isUnambiguouslyStartOfFunctionType`
holds, so the `=>` after a parenthesized return type such as
`(): (A | B) =>` belongs to the arrow function (TASK-499); `asserts` is a
type prefix only before a same-line name and `abstract` only before `new`,
and `declare`, `namespace`, and `module` begin a declaration only before
the tokens TypeScript's `isStartOfDeclaration` accepts (TASK-503). An
import-equals declaration's module reference is `require(…)` or an entity
name, never an expression (TASK-500). The lexer's own
regular-expression, JSX, and leading-`.` number decisions are the machine's
"an operand is expected here" (the `InputElementRegExp` goal of ECMA-262
§12), which the machine answers with its own transitions (TASK-494): it
offers the byte, as a punctuator, to a copy of its stack, so frames the
grammar has completed — a declaration's body, a control statement's body,
a statement only `else`/`catch`/`finally` could continue, one an automatic
semicolon ends — hand it down, and an operand is expected exactly when an
expression waiting for one receives it. Statement lists carry the `[Yield]`
parameter a function body sets, so `yield` is an operator only in a
generator. It also knows tt's statement-shaped constructs (`if let`,
let-else, `match` arms, `variant` bodies, construct bodies), whose shapes
TypeScript never has. Every consumer reads the facts instead of deriving a
boundary from the tokens around it: flow statement splitting and
function-scope queries, the parser's statement starts, `try`/`if let`
expression positions, pipeline heads and steps, and `match` host ambiguity,
the program-syntax projection's boundary semicolon, and `val`'s same-line
rule. The machine's oracle is SWC: for TypeScript input its statement spans
equal SWC's, and the lexer reads a regular expression or a JSX element
exactly where SWC parses one (`lexer/facts/tests.rs`, over the repository's TypeScript and
tt fixtures and the installed TypeScript package; `TTC_FACTS_CORPUS` adds
trees).

**One lexing per text (TASK-510).** Driving the machine makes a lexing
several times as expensive as the byte scan it replaced, so a compile lexes
each text once: the token stream the parse produced is the one Core
lowering, the semantic checks, and the program-syntax projection read (Core
lowering and the checks read the TypeScript-kind lexing, which is the same
stream for a `.tt` file and a second lexing only for `.ttx`), and the
engine's projection of a file parses it once for the compile and every
probe it records. The remaining whole-text lexings are of the host
projection and of the output, which the host syntax self-checks read; the
output's statement-boundary check reads the tokens its syntax check lexed
(TASK-512). A lexical question answers from the bytes alone where no token
could change the answer — no delimiter byte, TSX `<`, or conflict-marker
run for the host syntax check, no `await`, no `//` — without lexing.
`scaling_tests.rs` counts the parses and whole-text lexings of a compile and
of a projection.

For strict compilation, the host syntax checks cannot move behind SWC's parse of the same text
(TASK-512): SWC accepts some text they reject (an `export namespace` body
that lacks its `}` or holds a stray `)` parses without an error), and it
panics or backtracks on the rest (conflict-marker recovery, a TSX
namespaced member, deeply unbalanced type arguments), so each guarded text
is lexed before its parse.

Editor projection uses a separate, explicit SWC recovery mode (TASK-759).
Grammar routines retain missing expressions/types, synchronize failed
statements, and record missing delimiters with parser-local spans and owners.
Speculation rolls these records back with the parser checkpoint. Strict
parsing and output verification never enable this mode.

The host projection translates parser records through its source segments.
Known owners keep the existing evaluation/lowering model; tt values wholly
inside skipped host input become parser-owned editor holes. Missing closing
delimiters are materialized only in the editor input to lowering, so generated
statement preludes cannot be swallowed by unfinished calls. `EditorSource`
restores every source coordinate and splits copied mappings around inserted
bytes; those bytes have synthetic glue provenance, never authored mappings.
`ProjectionReport::syntax_repairs` preserves the original syntax cause without
suppressing independent type diagnostics in that production. `editor_only`
blocks build/declaration output. Engine and content mapper consume this report.

파일 표면은 `SourceKind::{TypeScript, Tsx}`로 컴파일 경계에서 정해지고 모든
단계에 전달된다. TSX 모드에서는 완전한 JSX element/fragment를 구조적으로
스캔한다. 태그·속성 이름·텍스트는 `JsxRaw`로 불투명하게 보존하고 `{...}`
expression container만 같은 렉서로 재귀 처리한다. 따라서 JSX 텍스트의 tt
키워드는 후보가 되지 않지만 expression container 안의 tt 구문은 일반 식과
같이 AST로 올라간다. SWC 입력·출력 검증도 같은 `SourceKind`를 사용한다.

### 3. `parser` — 무오류 구조 파싱

파서는 **에러를 내지 않는다**. 렉서가 만든 토큰 스트림을 토큰 커서
(`parser/cursor.rs` — `Copy` 핸들이라 서브파서가 복사본을 전진시키고
실패 시 호출자의 원본이 그대로 남는 것이 백트래킹의 전부다) 위에서
순회하며, 구문이 완전하게 파싱될 때만 AST 노드로 들어올리고, 조금이라도
어긋나면 그 후보를 verbatim 바이트 범위로 남긴다. "유효한 TS는 바이트
그대로 통과" 계약이 여기서 구현된다: 구문 여부는 순수하게 구조적 판단이고,
tt 수준 *에러*(중복 케이스 등)는 전부 sema의 몫이다. 중첩 코드(스크루티니,
arm body, 보간)는 같은 토큰 스트림의 부분 슬라이스로 재귀 파싱된다.

Editor features that ask about text still being typed read the parser's own
grammar, never a copy of it (TASK-492). `parser/partial.rs` answers which
pattern a token position is in — a match arm's pattern or the alternatives of
an `if let` or let-else — and which arms of a match body are already written.
It uses the same match head (`matches::match_body_open`), arm walk
(`matches::outline_arms`, which also delimits the strict and recovering arm
lists), arm pattern grammar, and single-pattern heads (`iflets::if_let_pattern`,
`lets::let_else_pattern`) that the sub-parsers commit with.

TypeScript `enum` 통과와 tt `variant` 소유권 규칙을 구분한다.
`const enum`/`declare enum`을 포함한 TypeScript 선언과 예약어 규칙도 파서 소관이다.

### 4. `sema` — 의미 검사

AST를 소스 순서로 깊이 우선 순회하며(노드 자체 규칙 → 자식 순),
첫 위반을 바이트 오프셋과 함께 `TtError`로 보고한다:

- variant: 중복 케이스 금지; 검증 활성 시 필드 타입이 TS 타입 조각으로
  파싱되는지(swc) 검사.
- match: 와일드카드 `_`는 마지막 arm; 중복 arm 금지.
- 소진성: sema는 **보고만** 한다. 후보 variant 표(로컬 > 임포트 > 내장),
  커버 규칙(가드·중첩 패턴 arm은 커버하지 않음), 튜플 곱집합은
  `analysis.rs`가 계산해 `Coverage`로 답하고(`match-analysis.md` §5),
  sema는 그 답을 순회 **종료 후** 위치 있는 에러로 옮긴다 (match가 variant
  선언보다 앞서도 무관). 알 수 없는 태그의 match는 검사하지 않는다 —
  ttc에 타입 정보가 없다.

에러 계층 계약이 여기서 지켜진다: 모든 tt 수준 에러는 sema가 직접 보고하고,
tsc에 위임하지 않는다.

### 4-1. `val` — 바인딩 수준 의미 검사

`val`(TASK-070)은 다른 tt 구문과 달리 **통과 영역의 TypeScript**에 대해
판단해야 한다: 어떤 바인딩이 `val`인지, 어떤 식이 그 바인딩에서 시작하는
경로를 변경하는지는 전부 AST가 의도적으로 불투명한 바이트 범위로 남겨 둔
코드 안에 있다. 그래서 이 검사만 AST가 아니라 **렉서가 만든 토큰 스트림**
위에서 돈다 (`val.rs`).

- The parser decides once whether a `val` is a modifier and records it as
  `Segment::ValModifier` with its kind (TASK-490). `val const|let|var` on
  one line is claimed from its tokens (`parser/vals.rs`). A `val <binding>`
  at the start of a `(`/`,` entry is a candidate; the host parse of the
  region's tt projection (`parser/host.rs`) keeps it only when the binding
  lands on a formal parameter, so `f(val [0])` and `c ? (val [0]) : w => w`
  stay TypeScript. `val::check`, the probes, and the engine read the AST's
  decision through `parser::val_modifiers` instead of re-deriving it.
- `val::check`는 같은 토큰 스트림을 한 번 훑으며 렉시컬 스코프 스택을
  쌓고(블록·함수 매개변수·`for` 머리·`catch`), 변경 경로의 루트 식별자를
  해석하고, 같은 파일에서 이름으로 선언된 함수의 시그니처로 호출 시점의
  변경 권한을 검사한다. 에러는 sema와 같은 `TtError`(바이트 오프셋)다.
- `parser::lex_and_parse` returns the token stream with the parse, so a
  file is still lexed once, and a file whose AST has no `val` modifier skips
  the pass entirely.
- **메서드 호출은 이 단계가 판정하지 않는다**(TASK-071). `x.set(k)`가 값을
  바꾸는지는 `x`의 타입에 대한 사실이고, 여기에는 타입이 없다 — 이름으로
  추측하면 같은 이름의 사용자 정의 API가 오탐으로 막힌다. 그래서 같은 워크가
  두 번째 모드(`val::method_calls`)로 그 호출들을 **질문**으로 수집하고,
  `ttc --types`가 실제 체커로 답한다. 리터럴 match의 타입 기반 소진성
  (`probe.rs`)과 같은 구조이고, 같은 원칙이다: 확정할 수 없으면 보고하지
  않는다.

이 단계가 sema와 분리된 이유는 **입력 자료가 다르기 때문**이다 — 규칙이
AST 노드 위에서 표현되면 sema, 통과 영역의 토큰 위에서 표현되면 val이다.
사용자에게는 둘 다 "ttc가 직접 내는 tt 수준 에러"로 동일하다.

### 5. `codegen` — 무오류 방출

sema를 통과한 AST에서 텍스트로의 순수 매핑이다. verbatim 구간은 원본
바이트를 그대로 복사하고, variant는 유니언 `type` + 생성자 `const`로 방출한다.
값을 만드는 match는 ProgramSyntax와 Evaluation IR이 정한 owner continuation에
따라 statement slot + `switch` 또는 expression-boundary intrinsic으로 방출한다. JSX
속성·자식 expression은 `Jsx` protocol frame의 순서 있는 eager position이며,
concise arrow의 expression body는 이름 있는 `ArrowExpression` host owner다. 이
두 모델이 JSX 속성의 부작용 순서와 화살표 함수의 렉시컬 범위를 보존한다(코드 형태의 규범은
[`../ai/tt.md`](../ai/tt.md)). `await` 감지는
AST에 남긴 원시 Span 위로 `scanner::contains_await`를 돌려 수행한다.

방출된 코드의 **레이아웃은 프린터가 소유한다**(TASK-198). 방출부는 공백을
쓰지 않고 구조만 말한다 — `push_break(depth)`는 "여기서 줄을 끝내고 이
lowering 안쪽 depth만큼에서 다시 시작"이라는 뜻이고, 실제 들여쓰기는
평탄화 시점에 정해진다: 기준(base)은 그 구문의 스코프가 열린 **줄의 선행
공백**이고(`Rope::anchored`가 구문마다 스코프를 연다), 거기에 depth만큼
단위 들여쓰기를 더한다. 그래서 함수 안에 놓인 match든 중첩 블록 안에 놓인
`result`든 자기가 대체한 문장과 같은 자리에서 블록이 시작한다. 조각은 자기
기준의 상대 depth로 조립되고 중첩은 `Rope::indented`가 옮긴다. **verbatim
구간은 재포맷하지 않는다** — 계약 1과 원본↔출력 매핑이 우선이므로,
레이아웃은 컴파일러가 쓴 글루에만 적용된다.

값을 감싸는 괄호도 규칙으로 정해진다. 초기화식·대입 우변·`return`
피연산자·인자 하나 — lower된 값이 놓이는 이 위치들에서 값보다 느슨하게
묶이는 연산자는 콤마뿐이므로, `scanner::has_top_level_comma`가 참일 때만
괄호를 남긴다. A postfix step's receiver (`x |> .trim()`) is a different
question, answered by `lexer::is_member_receiver`: the receiver must be one
primary expression (`(await p).then(g)` needs the parentheses, `s.trim()`
does not) that does not end in an optional chain, because a member access
written after `a?.b` joins the chain (ECMA-262 §13.3.9), so `a?.b |> .c`
is `(a?.b).c` (TASK-691). A call step's callee keeps a trailing chain
(`lexer::is_primary_expression`): `x |> o?.m` is `o?.m(x)`. 두 술어 모두 판정이 애매하면 괄호를 남기는
쪽으로 답한다 — 틀려도 잉여 괄호일 뿐 의미는 잃지 않는다.

방출은 내부적으로 Lit(컴파일러 글루)/Src(원본, 오프셋 유지)/Break·Scope
(레이아웃) 조각의 로프(`codegen/rope.rs`)로 조립된다 — 조각은 원본을 **빌려오고**(복사하지
않고) 평탄화 한 번에만 텍스트를 쓴다 — `compile()`은 평탄화한 텍스트만 쓰고,
언어 도구용 `emit_mapped()`(`ttc --emit-map`)는 원본↔출력 바이트 매핑까지
받아 에디터의 가상 TypeScript 문서에 쓴다(TASK-050). 이 도구 경로는
파싱+방출만 조합한다: sema·verify를 생략해 편집 중인 버퍼에도 무오류로
방출한다 — 진단이 `--check`의 몫이라는 에러 계층 계약은 그대로다.

**Statement boundaries survive lowering (TASK-496).** A statement the source
ends by automatic semicolon insertion stays ended in the target, whatever
emitter path wrote the text after it. The lexer lists those boundaries once
(`lexer::automatic_semicolons`, template interpolations included), and the
target file keeps them while it is finalized (`TargetFile::separate_statements`):
where a source piece carries the line break of such a boundary up to the next
statement's start, and the target continues with anything but that
statement's own source text, a `;` is written when the continuing text's
first token would join the previous statement (`lexer::continues_statement`,
ECMA-262 §12.10.2: `(`, `[`, a template, `+`, `-`, `/`, and TypeScript's `<`).
The program-syntax projection reads the same list for its boundary
semicolon. The self-check backs this up independently: after SWC parses the
output, `verify::verify_statement_boundaries` lexes it and requires a
boundary after every source statement end the output copies together with
its line break (`verify-failed` otherwise).

## 프로젝트 단위 실행 (드라이버)

`compile()`은 파일 하나짜리 순수 함수다 — 프로젝트 전체를 도는 일은 CLI
드라이버(`main.rs`)의 몫이고, 대량의 소스를 전제로 다음 규칙을 지킨다
(TASK-056).

- **파일당 한 번만 읽고 한 번만 스캔한다.** 입력을 읽으면서 `scan_module()`
  한 번으로 상대 `.tt` import 목록과 `@tt/std` 사용 여부를 함께 얻는다
  (`tt_imports()`/`imports_std()`를 잇달아 부르면 같은 파일을 두 번 판다).
- **임포트된 모듈의 선언 테이블은 실행당 한 번만 만든다.** 소진성 검사용
  선언 수집(`language.md` §9.3)은 파일마다 임포트 대상을 다시 읽고 다시
  파싱했었다 — 공유 모듈 하나를 N개 파일이 임포트하면 N번. 지금은 경로별로
  캐시하고, 임포트 대상이 이번 실행의 입력이기도 하면 이미 읽어둔 소스를
  그대로 쓴다.
- **파일 단위로 병렬 컴파일한다.** 컴파일은 파일 간 가변 상태를 공유하지
  않으므로 코어 수만큼 동시에 돈다(`-j`). 진단은 입력 순서로 모아서 내고,
  두 입력이 같은 출력 경로를 다투는 경우에만 쓰기를 부모 스레드로 되돌려
  순서를 지킨다 — **관측 가능한 결과는 스레드 수와 무관하게 동일하다.**

## Positions and lines (TASK-498)

Every stage reports byte offsets; a line and column exist only at a public
boundary, and every boundary converts through one line model,
`src/lines.rs`. It measures a text once (`LineMap`) under an explicit
policy, because which code points end a line is two facts, not one:

- `LineBreaks::Ecma` — ECMA-262's `LineTerminatorSequence`: LF, CR, CR LF,
  U+2028, U+2029. `tsc` reports diagnostics on these lines and ECMA-426
  source maps count them (§11.1.2.1), so compile errors, `ttc::line_col`,
  the typed engine's `Diagnostic` positions, the CLI renderer, source maps,
  sidecar maps, the banner's placement after a `#!` line, and indentation
  lookups (`line_start_before`) use them.
- `LineBreaks::Lsp` — LSP 3.17's end-of-line set: LF, CR LF, CR. Every
  position that goes to an editor or to the TypeScript language server
  (engine `Position`s, semantic tokens, hints, tt symbols and completions,
  the `--server` protocol) uses it, and `line_ending` reads a file's line
  ending from it.

Positions are measured in the decoded text (a byte-order mark is not a
column). The compiler's own column counts code points (what a rendered caret
lines up with); protocol and source-map columns count UTF-16 units.
`ProtocolPositions` converts a compiler position to a protocol one through
the byte both name. The line terminators themselves are recognized by the
scanner's ECMA-262 primitives, so multibyte text stays opaque.

## 타입 검사 실행 (엔진)

typed 모드(`--check-types`/`--types`/`--server`)는 배치 드라이버가 아니라
**엔진**(`src/engine/`)이 소유한다: `Project`(문서·projection 캐시·컴파일러
세션)가 장수명 상태를 들고, `Snapshot`(불변)이 한 패스의 단위이며, 결과는
TT-owned 타입으로 돌아온다. 설계 근거와 typescript-go 비교는
[`engine-architecture.md`](./engine-architecture.md)에 있다. 배치 빌드가
엔진 밖인 것은 tsgo가 배치 `tsc`를 project 시스템 밖에 두는 것과 같은
분리다 — 상태가 필요 없는 1회 실행에 세션 기구를 태우지 않는다.

## 기능 추가 가이드

| 변경 종류 | 손대는 단계 |
|-----------|-------------|
| 새 구문 | `ast`에 노드 추가 → `parser`에 구조 파싱 → `codegen`에 방출 (+ sema 검사 필요 시) |
| 새 의미 규칙/에러 | `sema`만 (통과 영역의 바인딩·식이 대상이면 `val`) |
| 방출 코드 형태 변경 | `codegen`만 (+ `docs/reference/language.md` 갱신) |
| New token-level recognition | `lexer` (token kinds and fusion, token facts) or `scanner` (byte primitives) |

어느 경우든 CLAUDE.md의 세 계층 테스트(compile / passthrough /
integration)와 레퍼런스 문서 갱신 규칙을 따른다.

## 재구성의 등가성 검증

이 재구성은 언어 표면을 바꾸지 않았다. 기존 테스트 전부(단위·계약·통합
59개 + doctest 4개) 통과에 더해, 재구성 전(HEAD) 바이너리와 후
바이너리를 22개 샘플 × (`-p`,
`-p --no-verify`)로 비교해 출력·에러 메시지·종료 코드가 바이트 단위로
동일함을 확인했다 (TASK-010 기록 참조).
