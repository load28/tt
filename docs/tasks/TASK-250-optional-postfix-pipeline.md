# TASK-250: 파이프라인 optional postfix step — 구조화 tail과 평가 계약

- **상태**: 완료
- **시작일**: 2026-08-27
- **완료일**: 2026-08-27
- **커밋**: `1f6d936`

## 목적

[GitHub Discussion #64](https://github.com/load28/tt/discussions/64)의 합의에 따라
`|> ?.` optional postfix step을 tt 0.4 문법으로 도입한다. 단순히
파서에서 `?.`를 허용하지 않고, postfix tail의 문법·평가·reference·
매핑 계약을 하나의 구조화된 모델로 만든다. 일반 postfix step과 optional
postfix step이 같은 receiver 및 검증 규칙을 소비하게 해 특정 예제용
분기나 codegen 휴리스틱을 만들지 않는다.

## 범위

- 포함:
  - `?.name | ?.[expr] | ?.(args)`로 시작하고 `.name | ?.name | [expr] |
    ?.[expr] | (args) | ?.(args)`를 반복하는 전체 postfix tail
  - 일반·optional postfix를 공통으로 표현하는 parser·AST·HIR·Core IR
    계약과 구조 검증
  - 잘못되거나 미완성인 optional tail의 원자적 tt 진단, 소유 노드,
    typed projection recovery
  - 파이프 입력 단일 평가, computed key·인자 단락 평가, optional
    method call의 `this`, 파이프 경계별 보호 범위
  - 일반·optional postfix의 공통 `push_receiver` 및 primary/non-primary
    receiver 괄호 계약
  - `flow`의 같은 step 모델 소비, VS Code 문법 강조·자동완성·hover·
    진단 매핑, 사용자·설계·AI 문서와 홈페이지
  - 컴파일 출력, 진단, TypeScript 통과·타입, 런타임, source map,
    editor, 실세계 코퍼스 회귀 검증
- 제외:
  - `|>?` 같은 null-aware 파이프와 후속 일반 step 자동 건너뛰기
  - optional construction, tagged template, private field를 포함한 합의 문법
    목록 밖 postfix 확장
  - JavaScript optional chaining의 nullish·오류 전파 의미 변경
  - Option·Maybe·Result 런타임, 타입 트릭, 자동 마이그레이션과 quick fix

## 합의에서 고정된 언어 계약

- `value |> ?.member`는 JavaScript optional chaining과 같은
  `(value)?.member`다. 각 `|>` 경계는 별도 step이며, optional step 하나가
  뒤의 다른 파이프 step을 보호하지 않는다.
- 프로퍼티·computed access·메서드·단독 optional call과 합의된 반복
  postfix tail 전체를 0.4에 포함한다.
- optional step 뒤의 일반 함수 step은 `undefined`를 받아도 실행하며,
  타입 불일치는 TypeScript checker가 담당한다.
- 미지원·미완성 tail은 부분 인식, 부분 변환, 원문 통과 없이
  optional step 전체를 tt 문법 오류로 거부한다.
- 일반·optional postfix는 같은 receiver 판정과 검증 행렬을 쓴다.
  optional-step 전용 precedence 판정기나 안전 표현식 긍정 목록은 만들지
  않는다.

## 의사결정

### 결정 1: postfix tail은 parser가 소유하는 하나의 문법 단위로 만든다

- **상황**: 현재 `src/parser/pipes.rs`는 첫 토큰이 `.`와 식별자인지만
  확인한 뒤 step 나머지를 평범한 `Program`으로 넘긴다. `?.`를 같은
  불리언에 더하면 완성된 tail, 미지원 tail, 회복 범위를 뒷 단계가
  분별할 수 없다.
- **검토한 대안**: `OptChain` 첫 토큰만 허용 / optional 케이스만 별도
  스캐너로 검색 / 포스트픽스 tail을 토큰 구조로 파싱.
- **선택과 근거**: 시작 형태와 반복 요소를 하나의 postfix-tail parser가
  검증하고, `PipeStep` 모델은 함수 적용과 구조화된 postfix tail을
  enum으로 구분한다. 현재의 `postfix: bool`은 제거한다. 이 경계가
  완성된 tt 구문만 클레임하는 계약과 원자적 진단의 단일 원천이다.
  정확한 enum·node 형태는 구현 중 AST→HIR→Core IR이 소비하는 정보를
  최소화하여 확정한다.

### 결정 2: 문법 유효성, 평가 조건, 괄호 판정을 서로 다른 책임으로 둔다

- **상황**: bare `super`처럼 optional-chain receiver가 될 수 없는 head와
  non-primary receiver처럼 괄호만 필요한 head는 서로 다른 문제다.
  computed key·인자·optional call은 또 별도의 조건부 평가 문제다.
- **검토한 대안**: codegen이 head 문자열과 tail 모양을 보고 한 번에 분기 /
  parser·의미 검증·evaluation lowering·codegen이 각자 자신의 질문만 소유.
- **선택과 근거**: parser는 tail 문법과 원자적 소유 범위, codegen 전
  검증은 optional receiver 유효성, evaluation 계층은 조건부 도달성과
  reference/효과, codegen은 이미 검증된 IR의 배치만 담당한다. 이렇게
  나누면 `super`만 거르는 codegen 예외, optional call에서만 작동하는
  `this` 복구 휴리스틱, key/인자를 미리 평가하는 폴백을 필요로 하지 않는다.

### 결정 3: 일반·optional postfix는 공통 receiver 계약만 소비한다

- **상황**: 현재 `push_receiver`는 `scanner::is_primary_expression`이 증명한
  receiver에서만 괄호를 생략하고 나머지와 미확정 Rope는 괄호로
  보호한다. 토론은 이 판정을 optional postfix에도 그대로 쓰도록
  합의했다.
- **검토한 대안**: optional step은 항상 `(E)?.tail`로 방출 / optional
  step용 primary 목록 추가 / 기존 `push_receiver` 공유.
- **선택과 근거**: 기존 helper를 공유한다. 공통 판정이 primary로
  확정한 값만 `value?.member`처럼 괄호를 생략한다. 나머지는
  `(E)?.member`로 방출한다. 일반·optional 입력 행렬을 동일하게
  실행해 두 형태의 파싱·실행·reference·source map을 같은 규칙으로
  검증한다.

### 결정 4: optional tail의 조건부 도달성은 Core Apply 계약으로 둔다

- **상황**: `src/program_syntax.rs`와 `src/evaluation_ir.rs`는 일반 TypeScript
  optional chain의 tail과 optional-call 인자를 이미 조건부 도달 영역으로
  모델링한다. 그러나 pipeline step은 원문 상태에서 독립된 TypeScript
  표현식이 아니므로, `?.[match …]`나 `?.m(match …)` 내부 값을 먼저
  물질화하면 JavaScript 단락 의미가 깨진다.
- **검토한 대안**: optional step에서 중첩 tt 구문을 금지 / key·인자별
  예외 lowering / optional postfix step의 모든 body를 조건부 expression
  boundary에 유지.
- **선택과 근거**: Core의 `ApplyMode::Postfix { optional }`이 step body의
  조건부 도달성을 소유한다. optional body는 Apply 앞 statement slot으로
  물질화하지 않고 tail 안의 expression boundary에서 낮춘다. key·인자별
  문자열 검사나 AST 모양별 분기 없이 중첩 `match`, `result`, `try`가 같은
  규칙을 소비한다.

### 결정 5: 검증은 예제 목록이 아니라 교차 행렬을 계약으로 삼는다

- **상황**: 프로퍼티 접근 하나만 검사하면 computed key, optional call,
  method reference, 비-primary receiver, 중첩 tt 구문에서 같은 원인의 회귀가
  남는다.
- **검토한 대안**: 보고된 예제만 compile 테스트로 추가 / receiver×tail×
  nullish 상태×중첩 구문의 행렬을 단계별 계약으로 고정.
- **선택과 근거**: 문법·출력·타입·런타임·매핑·editor 테스트를
  같은 행렬의 다른 관찰으로 구성한다. 일반·optional postfix에 같은
  receiver 행을 적용하고, nullish일 때 key·인자가 실행되지 않는지와
  존재할 때의 횟수·순서·`this`를 Node 런타임으로 검증한다. 출력·진단·
  source map은 전체 스냅샷으로 고정한다.

## 구현 계획

1. **문법·호환성 행렬을 먼저 고정한다.** 합의된 optional 시작·반복
   형태, 미지원·미완성 tail, 현재 일반 postfix가 받는 TypeScript
   postfix 형태, `flow` 첫·후속 step을 토큰 행렬로 작성한다. 기존
   일반 postfix 표면을 의도치 않게 좁히지 않는다.
2. **postfix-tail parser와 오류 노드를 만든다.** 토큰 균형과 해당
   항목의 시작·종료를 parser가 판정한다. `|> ?.`를 보면 optional
   postfix 의도를 원자적으로 소유하고, 정확한 스팬·진단 코드·pipeline
   owner·recovery를 남긴다.
3. **step 계약을 AST에서 Core IR까지 전달한다.** `postfix: bool`과
   `ApplyMode::{Call, Postfix}`에 숨어 있는 정보 손실을 제거한다. 각
   단계의 validator가 유효한 상태만 다음 단계로 넘기게 한다.
4. **evaluation owner와 TypeScript 방출을 연결한다.** primary 판정은
   `push_receiver`에서 공유하고 receiver 유효성은 그 전에 검증한다.
   조건부 key·인자의 중첩 tt lowering은 optional chain owner 내부에 남겨
   순서·단락·reference 의미를 보존한다.
5. **전체 소비자를 같은 문법 경계로 갱신한다.** source map·diagnostic
   snapshot, TypeScript 타입 진단 역매핑, completion probe, TextMate·semantic
   token, README 영문·한글,
   `docs/design/pipeline-operator.md`, `docs/design/program-lowering.md`,
   `docs/ai/tt.md`, 홈페이지 레퍼런스를 갱신한다.
6. **교차 행렬으로 검증한다.** primary/non-primary/미확정 receiver×
   property/computed/method/optional-call/mixed tail×present/nullish×plain/nested-tt
   입력에서 파싱, 방출, 타입, 런타임, 매핑을 확인한다. 일반
   postfix에도 동일 receiver 행렬을 적용한다.

## 작업 내역

- 2026-08-27: `./scripts/doctor`로 Rust·TypeScript 개발 환경을 확인했다.
- 2026-08-27: GitHub Discussion #64의 RFC 본문과 세 차례의 숙의 종합안을
  확인했다. 문법 범위, 원자적 거부, 공통 receiver 판정, 평가·
  reference·source map 검증을 고정 계약으로 옮겼다.
- 2026-08-27: 현재 pipeline parser, AST·HIR·Core IR, `push_receiver`, optional-chain
  whole-owner lowering, completion probe, 진단·recovery 계층을 조사했다.
- 2026-08-27: 조사 결과를 바탕으로 `postfix: bool`에 `optional`을 더하는
  방식을 버리고, 구조화 tail→유효성→evaluation owner→공통 receiver
  codegen 순서의 구현 계획을 수립했다.
- 2026-08-27: 태스크 문서 등록 뒤 `./scripts/ci` 전체 게이트를 실행해 변경 전
  기준선이 통과함을 확인했다.
- 2026-08-27: `origin/main`을 fetch하고 로컬 `main`을 fast-forward한 뒤 작업
  브랜치를 `a71eaea` 위로 rebase했다. main의 TASK-248·249와 충돌하지 않도록
  이 작업을 TASK-250으로 재번호화했다.
- 2026-08-27: optional postfix tail parser, 원자적 recovery·진단,
  `PipeStepKind`와 `ApplyMode`의 구조화 계약을 AST→HIR→Core IR에 추가했다.
- 2026-08-27: bare `super`를 토큰 구조로 분류해 의미 검증에서 거부하고,
  projection을 막아 잘못된 TypeScript 방출로 넘기지 않게 했다.
- 2026-08-27: optional tail의 중첩 tt 값을 조건부 expression boundary에
  유지해 computed key·인자 단락 평가와 method reference의 `this`를 보존했다.
- 2026-08-27: 출력·진단 snapshot, 타입·런타임·source map, completion probe,
  TextMate grammar, 영·한 문서와 홈페이지 레퍼런스를 갱신했다.

## 이슈 및 해결

### 이슈 1: 현재 step 모델은 optional postfix의 문법과 평가 정보를 전달할 수 없음

- **증상**: AST·HIR은 `postfix: bool`, Core IR은 `ApplyMode::Postfix`만 보존한다.
  parser도 일반 postfix의 첫 `.`와 식별자만 검증한다.
- **원인**: 기존 문법은 원문 tail을 receiver에 붙이는 것만으로 충분했고,
  tail 내부의 조건부 도달성과 미완성 소유 구조가 필요하지 않았다.
- **해결**: postfix tail을 parser 구조로 올리고 각 IR이 다음 계층에 필요한
  정보를 보존하게 한다. 완성되지 않은 상태는 codegen에 보내지 않는다.

### 이슈 2: optional tail 안의 tt 구문은 단순 선행 물질화로 의미가 깨짐

- **증상**: nullish receiver의 `?.[key]`와 `?.(args)`는 key·인자를 평가하지
  않아야 한다. 중첩 `match`를 receiver 검사 전에 slot으로 낮추면 이 계약을
  위반한다.
- **원인**: postfix step을 단지 accumulator 뒤에 붙는 문자열로 보면 tail
  내부의 평가 구간을 IR에서 표현할 수 없다.
- **해결**: optional 여부를 Core `ApplyMode`에 보존하고, optional postfix의
  body는 statement form이 있어도 tail 내부 expression boundary에서 낮춘다.
  key·인자·callee 모양별 예외는 추가하지 않았다.

### 이슈 3: 로컬 `gh` 인증으로 Discussion을 읽을 수 없었음

- **증상**: `gh auth status`가 기본 계정의 토큰을 invalid로 보고했다.
- **원인**: 로컬 GitHub CLI 토큰이 유효하지 않았다.
- **해결**: 공개 Discussion HTML을 읽어 RFC 본문과 합의 댓글을 확인했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`
- [x] `UPDATE_EXPECT=1 cargo test --test snapshot` 후 출력·진단 diff 검토
- [x] TypeScript 파싱·타입 행렬과 Node 런타임 평가 순서·`this` 행렬
- [x] source map·LSP 진단·completion probe·TextMate 문법 테스트
- [x] `TTC_CORPUS_FULL=1 cargo test --test corpus` — 594개 중 유효 TS 534개 보존
- [x] `bun run typecheck`, `bun run build` — 홈페이지 37개 경로 prerender

## 결과

`?.name`, `?.[key]`, `?.(args)`와 반복 postfix tail을 하나의 parser 소유
문법으로 도입했다. optional 여부는 AST→HIR→Core IR에 보존되며, 공통 receiver
방출과 Core 조건부 도달성 계약이 단일 평가·단락·`this`를 지킨다. 미완성·
미지원 tail은 한 진단과 recovery node로 처리된다. 타입·런타임·매핑·에디터·
문서·홈페이지와 전체 로컬 CI가 통과했다.

변경 파일: `src/{ast,parser,hir,core_ir,sema,diagnostics,codegen,engine}/`,
`tests/{compile,integration,emit_map,native,fixtures}/`, `editors/vscode/`,
`README.md`, `README.ko.md`, `docs/{ai,design,tasks}/`, `website/src/`.
