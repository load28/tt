# TASK-160: SWC whole-owner 기반 RL→TS 최적 lowering

- **상태**: 완료
- **시작일**: 2026-08-22
- **완료일**: 2026-08-24
- **커밋**: 02a279a, 2f76582, 1e03992, 1dbdc2b, a644a7c, d2dbe6b

## 목적

SWC 전체 AST를 실제 변환 골격으로 사용한다. 모든 RL 구문을 포함한 TypeScript
owner를 평가 순서대로 구조화하고, 각 Core primitive를 host 문맥에 가장 자연스러운
TypeScript 제어 흐름·표현식·선언으로 낮춘다. IIFE 제거는 이 최적 lowering의 한
결과로 취급한다.

## 범위

- 포함: SWC owner identity와 span, host expression 선형화, 공통 value slot
  continuation, 단락·반복·호출 reference 보존, `Decision`·`Propagate`·`Apply`·
  `Adt`·source edit의 TS-native lowering, source mapping과 기존 진단 보존,
  불필요한 wrapper/helper/temporary 제거 validator
- 제외: 사용자 TypeScript가 원래 작성한 IIFE 변경, 출력 포매팅 전면 변경,
  RL 언어 표면 변경

## 2026-08-24 감사 — 남은 아키텍처의 현재 상태

이 절은 TASK-198/TASK-199가 끝난 `c5f3e25` 기준으로 TASK-160의 남은 범위를
코드에서 다시 확인한 결과다. 조사 대상은 `AGENTS.md`,
`docs/design/compiler-architecture.md`, `docs/design/lowered-ir.md`,
`docs/design/program-lowering.md`, TASK-198/199 문서,
`src/program_syntax.rs`, `src/evaluation_ir.rs`, `src/core_ir/`,
`src/codegen/core.rs`, `src/codegen/rope.rs`, `src/verify.rs`,
그리고 compile/integration/native/emit-map 테스트다.

### A. 문서에만 있고 코드에는 없는 계약

`program-lowering.md` §11이 요구하는 여덟 validator 중 실제로 실행되는 것은
넷뿐이다.

| validator | 상태 |
|---|---|
| `validate_projection` | `ProjectionBuilder`가 segment 표를 만들고 span 왕복은 `source_span_for_projection`이 보장 — 사실상 존재 |
| `validate_program_syntax` | `ProgramSyntax::validate` — 존재 (span 범위·parent 유무·owner 표) |
| `validate_eval` | `EvaluationFile::validate` — 존재 (종료·도달·결과 정의) |
| `validate_order` | **없음** |
| `validate_reference` | **없음** |
| `validate_origin` | `TargetFile::validate`가 일부 검사하지만 `debug_assert!`로만 실행 — release 경로에서는 아무것도 검증하지 않는다 |
| `validate_source_preservation` | **없음** |
| `verify_output` | `verify.rs::verify_output` — 존재 (`--no-verify`로 생략 가능) |

`Effects`(§9)는 타입도 사용처도 없다. 효과 기반 최적화 순서(§9)도 구현되지
않았다.

### B. 암묵적 fallback으로 남은 경로

`EvaluationOwner`가 target capability를 고른다는 결정 3·11과 달리, 실제
capability 판정은 codegen의 `TargetRewritePlan::build`에 있다
(`src/codegen/core.rs`). `can_structure_value_expr`,
`compose_schedule_is_structurable`, `schedule.steps().is_empty()`,
`frequency == Once` 같은 술어가 거기서 host 의미를 다시 판단하고, 조건에
걸리지 않으면 값은 조용히 `emit_expr` → `$tt_expr` 경계로 떨어진다. 이는
명시적인 capability가 아니라 이름 없는 fallback이고, 불변 계약 8
("codegen은 의미를 새로 판단하지 않는다")을 위반한다. `LoweringPlan`에는
"이 값은 왜 expression boundary인가"를 표현하는 타입이 없다.

### C. 동일한 사실을 여러 계층이 중복 계산하는 부분

- `can_structure_value_expr`(codegen)는 Core 모양에 대한 술어인데 codegen이
  소유한다. Evaluation IR은 같은 질문을 하지 않고 codegen만 한다.
- `EvaluationContext.frequency`는 `EvaluationOwner`(함수/모듈) 기준으로
  계산되는데, 실제 prelude가 삽입되는 곳은 `HostOwner`(문장)다. 두 기준이
  다른데 하나만 계산한다 — 아래 이슈 14의 원인이다.

### D. 조사 중 확인한 실제 결함

감사는 문서 대조에 그치지 않고, 위 구조적 결함이 실제 출력에서 관측되는지를
`cargo run -- -p --no-banner <file>`로 확인했다. 세 건 모두 재현된다.

1. **loop header의 값이 loop 밖으로 hoist된다** (이슈 14).
2. **conditional region 안에서 capture한 slot이 region 밖에서 읽힌다**
   (이슈 15) — 생성 TypeScript가 컴파일되지 않는다.
3. **capture span이 tt 값 span과 겹친다** (이슈 16) — 생성 TypeScript가
   파싱되지 않고 원본 바이트가 중복·누락된다.

셋 다 "validator가 없어서 잡히지 않은 계약 위반"이며, 이번 작업의 validator
설계는 이 세 계약을 먼저 이름 있는 타입으로 만드는 데서 출발한다.

### E. 테스트는 있지만 compiler validator가 없는 불변식

`tests/passthrough.rs`는 "유효한 TypeScript는 바이트 그대로 통과"를 56건의
입력으로 확인하지만, 컴파일러 자신은 원본 보존을 전혀 검사하지 않는다.
`tests/compile.rs`의 출력 스냅샷도 같은 성격이다 — 코퍼스에 없는 입력 모양은
아무도 지키지 않는다. 이슈 16이 그 증거다.

## 의사결정

### 결정 1: SWC AST는 분류기가 아니라 host rewrite와 최적화의 단일 골격이다

- **상황**: TASK-159는 SWC가 직접 return을 증명한 경우만 Core 조각을 바꿨다.
  이 방식은 호출 인자·선언 initializer·단락 표현식마다 예외가 늘어난다.
- **검토한 대안**: 위치별 최적화는 작게 적용할 수 있지만 완전한 AST 소유의
  목적을 달성하지 못한다. owner 전체를 선형화하면 구현량은 늘지만 모든 RL
  값 구문이 동일한 continuation 규칙을 사용한다.
- **선택과 근거**: SWC가 찾은 최소 실행 owner 전체를 Evaluation IR로 낮추고,
  target은 `prelude + value slot + rewritten host`를 기본 형태로 사용한다. 이후
  효과·사용 횟수·continuation을 근거로 slot과 temporary를 제거하거나 직접
  `return`·대입·분기로 합친다. Core primitive별 wrapper 선택은 허용하지 않는다.

### 결정 2: projected node와 source-backed owner를 별도 origin으로 관리한다

- **상황**: statement/item placeholder가 SWC 내부에 가짜 statement를 만들기 때문에
  가장 가까운 AST statement가 항상 원본 변환 owner인 것은 아니다.
- **검토한 대안**: statement 구문만 source span으로 fallback하면 해당 구문 전용
  예외가 된다. 모든 projected span을 선형 역투영하면 길이가 다른 placeholder 내부
  위치를 거짓 source 위치로 만들게 된다.
- **선택과 근거**: projection segment를 `Copied | Placeholder` origin으로 타입화하고,
  AST owner stack에서 원본으로 완전히 역투영되는 가장 안쪽 owner를 선택한다. 이
  규칙은 expression·statement·item에 동일하게 적용된다.

### 결정 3: 최소 owner와 타입화된 target capability를 사용한다

- **상황**: 호출 인자·매개변수·클래스 초기화를 boundary로 분류하면 legacy wrapper가
  계속 남고 whole-AST 소유 목적이 사라진다.
- **검토한 대안**: 이유가 있는 boundary closure를 유지하는 방식은 의미 보존에는
  보수적이지만 최적 TS lowering을 완료할 수 없다. owner 전체를 변환하면 reference와
  독립 실행 환경을 IR에서 직접 보존해야 하지만 구문별 fallback이 사라진다.
- **선택과 근거**: 모든 값은 안정적인 `HostOwnerId` 아래 `PlannedValue`가 되고,
  소비 방식은 `ValueSlotId`와 host continuation으로 표현한다. statement를 허용하는
  호출·선언·return owner는 owner transform에 들어간다. 표준 TypeScript가 statement를
  허용하지 않는 매개변수·class field는 분석 실패 fallback이 아니라 명시적인
  expression-boundary target capability를 사용한다.

### 결정 4: 중첩 region은 부모의 value continuation을 직접 상속한다

- **상황**: initializer match를 statement control flow로 구조화해도 arm 값이 다시
  match이면 자식만 expression emitter로 돌아가 IIFE가 남았다.
- **검토한 대안**: 중첩 match만 재귀 출력하면 현재 사례는 해결되지만 다른 value
  primitive가 들어올 때 같은 분기가 반복된다. 자식 값을 먼저 임시 변수로 만든 뒤
  부모에 대입하면 불필요한 join과 이름이 늘어난다.
- **선택과 근거**: expression-valued 자식 region은 부모의 `ValueDestination::{Expression,
  Assign}`
  continuation을 그대로 받는다. 자식의 모든 정상 leaf가 같은 continuation을 소비하므로
  별도 expression boundary가 필요 없고 CFG의 value edge와 target 제어 흐름이 일치한다.

### 결정 5: concise arrow는 별도 closure가 아니라 ArrowReturn continuation이다

- **상황**: `(...) => match ...`는 이미 함수 실행 환경 안에 있지만 일반 `Compose`로
  분류되어 또 다른 closure를 만들었다.
- **검토한 대안**: 출력 문자열에서 `=>`를 찾는 방식은 타입·주석·괄호 변화에 취약하다.
  arrow 전체를 SWC printer로 다시 출력하면 원본 source piece 보존 계약을 깨뜨린다.
- **선택과 근거**: SWC `ArrowFunctionBody::Expr` parent edge를 `ArrowReturn` continuation으로
  타입화한다. 기존 arrow의 expression 조각만 `block + value slot + explicit return`으로
  교체하고 prefix·parameter·type annotation·suffix는 원본 조각을 그대로 사용한다.

### 결정 6: value continuation은 destination과 wrapper의 합성이다

- **상황**: `result` region은 실패 값은 그대로 전달하지만 성공 값만 `Ok(value)`로
  감싸야 한다. 단순 `Assign(target)`만으로는 nested decision/result leaf가 이 차이를
  표현할 수 없다.
- **검토한 대안**: result 전용 slot emitter는 빠르지만 decision과 propagation이 다시
  서로 다른 제어 흐름을 갖는다. 성공 값을 별도 temporary에 모은 뒤 `Ok`를 만들면
  중첩 단계마다 불필요한 join이 생긴다.
- **선택과 근거**: `ValueContinuation`을 `ValueDestination::{Expression, Assign}`와
  `ValueWrapper::ResultOk`의 합성으로 정의했다. 실패 edge는 기존 continuation을 소비하고
  성공 edge만 wrapper를 추가한다. wrapper는 중첩될 수 있으므로 nested result도 타입화된
  동일 규칙을 사용한다.

### 결정 7: Sequence는 선행 효과와 최종 value를 분리하는 순차 region이다

- **상황**: result의 최종 match가 HIR `Sequence` 안에서 trivia·선행 statement와 함께
  보존되어 자식 continuation으로 합성되지 않았다.
- **검토한 대안**: 단일 expression만 든 sequence를 투명 처리하면 주석이 추가되거나
  선행 statement가 있는 순간 다시 wrapper가 생긴다.
- **선택과 근거**: 마지막 value statement 앞의 모든 statement를 순서대로 실행하고,
  마지막 value만 부모 continuation으로 전달한 뒤 후행 source trivia를 보존한다. 이는
  result 전용 처리가 아니라 모든 Core sequential value region의 실행 규칙이다.

### 결정 8: host return은 분기별 return이 아니라 명시적 value join을 사용한다

- **상황**: result의 실패·성공 edge를 각각 직접 return하면 실행 의미는 같지만 TypeScript가
  각 edge를 별도 표현식으로 검사한다. 기존 단일 result 표현식의 전체 contextual type과
  진단 경계가 분해되었다.
- **검토한 대안**: 진단 문자열을 result 전용으로 합치는 방식은 생성 형태에 의존한다.
  출력 IIFE를 유지하면 표현식 경계는 보존되지만 이번 구조 전환의 목적에 어긋난다.
- **선택과 근거**: 모든 host value에 충돌 없는 `ValueSlotId`를 할당한다. 구조화된 edge는
  slot에 값을 기록하고 하나의 원본 TypeScript return이 join 결과를 소비한다. slot 소비
  지점도 Core value의 source anchor를 상속하므로 전체 contextual type과 원문 진단 범위가
  함께 유지된다.

### 결정 9: host 평가 순서는 AST node의 ordered protocol로 합성한다

- **상황**: 호출 인자만 선형화하면 배열·객체·대입·시퀀스·단항식·template에서 같은
  IIFE 문제가 반복되고, 한 owner에 RL 값이 둘 이상 있으면 owner 전체를 구조화하지 못한다.
- **검토한 대안**: 구문별 출력 함수를 추가하면 현재 예제는 처리하지만 평가 순서와
  conditional 실행을 중복 구현하게 된다.
- **선택과 근거**: SWC node가 실제로 평가하는 child span을 `Ordered` protocol로 만들고,
  선행 source와 RL slot 의존성을 하나의 owner schedule로 합성한다. 같은 source 입력은
  owner 안에서 한 slot을 공유하며 여러 RL 값도 한 prelude에서 source 순서로 구조화한다.

### 결정 10: block arm의 return은 Core join exit로 낮춘다

- **상황**: block arm은 기존 익명 함수의 `return`에 의존해 match 값을 만들었으므로
  statement lowering에서 그대로 복사할 수 없었다.
- **검토한 대안**: block arm만 계속 expression boundary에 두면 동일한 Decision이 arm
  모양에 따라 다른 backend를 사용한다. 문자열로 `return`을 찾으면 중첩 함수의 return을
  구분하지 못한다.
- **선택과 근거**: decision projection이 arm 내부 TypeScript island를 SWC에 노출한다.
  SWC 함수 깊이로 현재 arm에 속한 `ReturnStmt`만 `HostExit`로 수집하고, target은 이를
  continuation assignment와 labeled break로 바꾼다. return 없는 완료 경로는 기존처럼
  `undefined` 값을 생산한다.

### 결정 11: statement 불가 owner는 이름 있는 expression boundary를 사용한다

- **상황**: parameter default와 class field initializer에는 표준 TypeScript statement를
  삽입할 위치가 없다. owner 밖으로 slot을 올리면 parameter scope, `this`, `arguments`,
  field 초기화 시점이 달라진다.
- **검토한 대안**: 매개변수·constructor를 전면 재작성하면 body declaration visibility,
  함수 `length`, derived `super`, field define semantics까지 별도 JS lowering이 필요하다.
  기존 익명 IIFE는 의미는 맞지만 출력과 번들러 경계를 계속 구문마다 만든다.
- **선택과 근거**: EvaluationOwner가 expression-only이면 파일당 하나의 hygiene된
  `$rl_expr<T>(run: () => T): T` target intrinsic을 선택한다. Core CFG callback은 원래
  평가 위치에서 실행된다. 이는 조용한 legacy fallback이 아니라 검증되는 target
  capability이며, 생성 IIFE는 0개다.

### 결정 12: 평가 빈도는 host owner 기준으로도 계산한다

- **상황**: `EvaluationContext.frequency`는 `EvaluationOwner`(함수/모듈) 기준
  빈도인데, statement lowering이 prelude를 삽입하는 위치는 `HostOwner`(문장)다.
  값이 `while (…test…)`처럼 loop **머리**에 있으면 host owner가 loop 문장
  자신이므로, 함수 기준으로 "Repeated"라는 사실을 알아도 그 사실이 hoist 가능
  여부를 답해주지 못한다.
- **검토한 대안**: (a) 기존 `frequency`의 의미를 host owner 기준으로 바꾼다 —
  한 이름이 두 질문에 쓰이던 것을 하나로 줄이지만, 함수 기준 빈도를 쓰는
  판단(예: 실행 환경 분류)이 조용히 의미를 바꾼다. (b) codegen에서 owner가
  loop 문장인지 확인한다 — 구문 모양을 codegen이 다시 판단하는 금지된 방식이다.
- **선택과 근거**: (c) `HostOwner`까지의 parent path에서 계산한 별도의 이름 있는
  사실 `EvaluationContext.host_frequency`를 추가한다. `ProgramSyntax`가 host
  owner를 고르는 바로 그 자리에서 계산하므로 두 사실의 기준이 어긋날 수 없다.
  이를 위해 `ProjectedHostOwner`에 owner를 push한 시점의 parent path 길이를
  기록한다 — owner와 값 사이의 edge만 보는 유일한 방법이다.

### 결정 13: target capability는 Evaluation IR이 정하고 codegen은 소비만 한다

- **상황**: capability 판정이 codegen에 흩어져 있고, 조건에 안 맞으면 이름 없는
  fallback으로 `$tt_expr`에 떨어진다 (감사 B).
- **검토한 대안**: codegen의 술어들을 그대로 두고 validator만 추가하면, validator가
  검사할 "계획"이 계획이 아니라 emitter의 부수 효과가 된다. 검증 대상이 없다.
- **선택과 근거**: `PlannedValue`에 `TargetCapability::{StatementRegion,
  ExpressionBoundary { reason }}`를 넣는다. `ExpressionBoundaryReason`은 왜
  statement lowering이 불가능한지를 이름으로 남긴다 — owner가 statement를 받지
  못함, host owner 기준 반복, conditional region 안의 capture, capture span 겹침,
  Core 값에 statement 형태가 없음. codegen은 이 값을 읽기만 한다. Core 모양 술어
  `can_structure_value_expr`는 `CoreFile::has_statement_form`으로 Core IR에
  옮겨 한 곳이 소유한다.

### 결정 14: conditional region 안에는 source capture를 두지 않는다

- **상황**: schedule의 conditional step은 action을 `if (…) { … }`로 감싼다. 그런데
  더 안쪽 frame의 source capture(`const $tt_vN = (…)`)도 그 블록 안에 들어가는
  반면, 그 slot을 읽는 host 표현식은 블록 **밖**에 그대로 남는다. `a && f(match …)`가
  `Cannot find name '$tt_v1'`로 컴파일되지 않는 이유다 (이슈 15).
- **검토한 대안**: (a) capture를 `let`으로 owner 수준에 선언하고 블록 안에서 대입 —
  스코프는 맞지만 TypeScript가 definite assignment를 증명하지 못해 타입이
  `T | undefined`가 된다. 이 태스크가 금지한 바로 그 결과다. (b) optional call만
  예외 처리 — 구조적으로 같은 `&&`/`||`/`??`/삼항에서 같은 버그가 남는다.
- **선택과 근거**: 계약을 한 문장으로 세운다 — **생성된 conditional region 안에서
  capture한 slot은 그 region 밖에서 읽힐 수 없다.** host 표현식은 원래 자리에
  그대로 남으므로 region 밖이다. 따라서 conditional 깊이 1 이상에서 source
  capture가 필요한 값은 statement lowering 대상이 아니다
  (`CaptureInsideConditionalRegion`). 이 규칙은 조건 입력 자체만 capture하는
  `x ? match … : y`, `f?.(match …)`, `a && match …`를 그대로 통과시키고,
  안쪽 frame이 capture를 요구하는 `a && f(match …)`만 거른다.
  이 규칙은 임시방편이 아니라 **capability의 전제**다: 뒷단계에서 conditional
  operation 전체(활성 분기의 host source 포함)를 region이 소유하게 되면, capture의
  host 사용처도 region 안으로 들어오므로 같은 규칙이 그대로 통과시킨다.

### 결정 15: source capture는 tt 값·다른 capture와 겹칠 수 없다

- **상황**: `g(a && match …, match …)`에서 두 번째 값의 "앞선 인자" 입력 span이
  첫 번째 tt 값을 포함한다. 그 span을 그대로 capture하면 `match` 원문이 생성
  코드에 복사되고, 겹치는 replacement 때문에 원본 바이트가 중복·누락된다
  (이슈 16).
- **검토한 대안**: capture 텍스트에서 tt 값을 찾아 치환한다 — 문자열 기반
  휴리스틱이고, 중첩된 값마다 예외가 늘어난다.
- **선택과 근거**: capture span은 어떤 tt 값 span과도, 다른 capture span과도
  겹치지 않아야 한다는 계약을 세우고, 겹치는 값은 statement lowering에서 제외한다
  (`CaptureOverlapsValue`). 이 계약은 `validate_source_preservation`이 target에서
  독립적으로 다시 검사한다.

### 결정 16: validator 실패는 이름 있는 불변식을 가진 구조화된 내부 오류다

- **상황**: 내부 오류가 `panic!("internal compiler error: …")` 문자열로 흩어져 있어
  어떤 단계의 어떤 불변식이 깨졌는지 타입으로 남지 않는다.
- **검토한 대안**: 각 validator가 자기 enum을 반환하고 호출자가 문자열로 합친다 —
  단계·주체·위치가 여전히 문자열이 된다.
- **선택과 근거**: `src/ice.rs`에 `LoweringStage`, `Invariant`, `LoweringSubject`,
  `InternalCompilerError`를 둔다. 실패한 단계, 위반한 불변식(이름 있는 enum),
  owner/Core root/operation/ValueId/ValueSlotId/BlockId, source span, origin
  chain을 모두 타입으로 담는다. 표시 문자열은 이 타입에서 파생될 뿐이다.

### 결정 17: 조건부 operation은 전체가 하나의 region으로 lowering된다

- **상황**: validator 단계를 세운 뒤 7.4 기준으로 재검증하자, 기존 statement
  lowering이 `&&`/`||`/`??`/삼항/optional call 전부에서 지시가 금지한 패턴
  ("값만 slot으로 승격하고 원래 조건 문법을 유지")이었음이 드러났다. 네 형태
  모두 tsgo가 `T | undefined` 새 오류를 보고했고(계약 7 위반), optional call은
  선행 인자를 nullish 여부와 무관하게 평가했다(평가 횟수 위반). spread 인자는
  `ExprOrSpread.span()`을 그대로 capture해 `(...xs)`라는 잘못된 TypeScript를
  만들었다.
- **여섯 질문의 답**: ① 사실 계산 책임 — 조건 연산의 구조(활성 branch,
  건너뛰는 branch, 전체 인자 목록, spread, type args)는 SWC 구문 사실이므로
  ProgramSyntax가 `ConditionalFacts`로 계산한다. ② 구조적으로 같은 입력 —
  활성 branch가 조건부로 평가되는 모든 TypeScript 연산: `&&`·`||`·`??`·삼항·
  optional call(직접·member callee). ③ 이름 있는 타입 —
  `HostEvaluationStep.conditional: Option<ConditionalFacts>`와 plan의
  `PlannedConditionalOperation`. ④ 검증 — validate_order(조건부 region 밖
  capture 금지 규칙은 그대로; op에 소비된 값은 region이 소유),
  validate_reference(member callee는 receiver가 있어 `.call`로 보존될 때만),
  그리고 타입 검사 테스트. ⑤ 회귀 — 각 형태의 tsgo 타입체크 + 단락 시 인자
  미평가 runtime trace. ⑥ mutation — else-branch 대입을 제거하면 타입 테스트가,
  선행 인자 capture를 검사 밖으로 옮기면 trace 테스트가 실패한다.
- **검토한 대안**: (a) 조건부 step이 있는 값을 전부 boundary로 보낸다 —
  의미는 맞지만 완료 기준 6·7(optional operation의 전체 CFG 표현,
  불필요한 boundary 제거)을 포기한다. (b) 값 slot을 `let r: T | undefined`로
  두고 소비 지점에 non-null 단언 — 타입 assertion으로 오류를 숨기는 금지
  방식이다.
- **선택과 근거**: 값의 schedule이 정확히 "가장 안쪽 step 하나가 Conditional"
  이고 활성 branch가 값 자신(투명 wrapper 허용)일 때, 그 조건 연산 **전체**를
  하나의 region으로 lowering한다: 연산의 parent span을 result slot으로
  대체하고, region은 `조건/callee 평가 → (optional이면 nullish 검사) →
  활성 branch에서 선행 인자 순서 평가·값 region·후행 인자 → 호출/값을
  result에 기록 → 비활성 경로는 조건 값(논리), 건너뛴 branch(삼항),
  `undefined`(optional call)를 result에 기록`으로 방출한다. 모든 경로가
  result를 대입하므로 TypeScript가 `T | undefined`를 만들지 않고, 인자는
  검사 안에서만 평가된다. member callee는 TypeScript 컴파일러 자신의
  downlevel 방출과 같은 `receiver 한 번 평가 + callee.call(receiver, ...)`
  로 this를 보존한다. 같은 parent를 공유하는 여러 tt 값(삼항 양쪽,
  여러 인자)은 하나의 operation으로 묶인다. 이 형태로 구조화할 수 없는
  조건 연산(중첩 조건, 사이에 낀 eager frame의 capture)은 이유 있는
  `ExpressionBoundary`로 남는다 — "operation 전체를 구조화할 수 있을 때만
  statement lowering을 선택한다"는 지시 그대로다. 아울러 호출·생성 인자의
  protocol span을 `ExprOrSpread.span()`에서 표현식 span + spread 사실로
  바꿔 spread capture가 표현식만 capture하고 `...`는 원 위치에 남긴다.

### 결정 18: Effects는 ProgramSyntax가 소유하고 최적화만 소비한다

- **상황**: §9의 Effects 모델을 어느 계층이 소유할지 결정해야 했다.
- **검토한 대안**: Core 의미 계층 — tt 구문의 효과는 알지만 host TypeScript
  표현식은 opaque다. Evaluation IR — 실행 사실의 소비자이지 구문 판정자가
  아니다.
- **선택과 근거**: 효과는 host TypeScript **표현식**에 대한 구문 사실이므로
  SWC AST를 소유한 ProgramSyntax가 `Effects`(may_read_mutable/may_write/
  may_call/may_throw/may_suspend/may_allocate/requires_reference)를 계산해
  protocol input마다 붙인다. 판정은 극도로 보수적이다: 평가가 관측 불가능함이
  구문만으로 증명되는 형태 — 일반 리터럴(정규식 리터럴은 평가마다 새 객체를
  할당하므로 제외), 투명 TS wrapper 아래의 그것 — 만 `NONE`이고, 식별자는
  TDZ와 가변 바인딩 때문에, 알 수 없는 모든 표현식은 무조건 `ANY`다. 소비는
  단 한 곳 — resolve_schedule의 capture 생략(`PlannedEvaluationInput::Stable`)
  — 이며 correctness는 Effects 없이도 성립한다(모두 capture해도 맞다).
  이것이 §6의 증명 기반 "한 번만 사용하는 안전한 source capture 제거"다:
  inert한 입력의 유일한 역할은 순서 보존이었고, 관측 불가능한 평가는 순서를
  가지지 않으므로 slot과 capture를 함께 제거해도 어떤 trace도 변하지 않는다.

### 결정 19: 값 region의 exit target은 하나이고, label은 필요할 때만 존재한다

- **상황**: TASK-199가 기록한 `do { … } while (false)` + label block 이중 중첩이
  남아 있었다. if-chain dispatch에서는 두 구조가 같은 region에 두 개의 exit
  target을 만들고, 실제로는 label이 쓰이지 않는 경우에도 항상 방출됐다
  (`$tt_y_$tt_v0: { … do { … } while (false); }`).
- **여섯 질문의 답**: ① 사실 계산 책임 — "이 exit의 `break`가 삼켜지는가"는
  arm body의 TypeScript 구문 사실이므로 ProgramSyntax가 SWC walk에서 계산한다.
  ② 구조적으로 같은 입력 — block arm의 `return`이 rewrite되는 모든 match
  (switch·if-chain·tuple·literal). ③ 이름 있는 타입 — `HostExit.captured_break`.
  ④ 검증 — 출력 테스트(label 유무)와 runtime 테스트(loop 안 exit). ⑤ 회귀 —
  `a_block_arm_exit_leaves_the_region_from_inside_a_loop` 등 2건. ⑥ mutation —
  M8(항상 false)·M9(항상 true) 양방향 모두 실패 확인.
- **검토한 대안**: (a) 출력에서 label 문자열을 찾아 지우는 사후 정리 — 지시가
  금지한 문자열 기반 처리다. (b) dispatch 종류로 분기(switch면 label, if-chain이면
  do-while) — 두 규칙이 되고, arm body가 loop를 쓰면 if-chain도 label이 필요하므로
  틀린다.
- **선택과 근거**: 원리는 하나다 — **region이 이미 만드는 dispatch(if-chain의
  `do { … } while (false)`, 또는 `switch` 자신)가 가장 가까운 `break` 대상이므로,
  label은 rewrite된 exit이 그 사이의 loop/`switch`에 삼켜질 때에만 필요하다.**
  `ParentCollector`가 loop·switch를 지나며 `break_capture_depth`를 세고, exit이
  region 진입 시점보다 깊으면 `captured_break`가 참이 된다. codegen은 이 사실만
  소비해 label을 방출할지 정하고, exit rewrite는 label이 없으면 unlabeled
  `break;`를 쓴다. 이로써 어떤 region도 exit target을 둘 갖지 않는다.

## 작업 내역

- 2026-08-22: TASK-160을 등록하고 SWC whole-owner cutover를 시작했다.
- 2026-08-22: projection origin을 `Copied | Placeholder`로 분리하고 source-backed
  최소 owner 선택을 일반화했다.
- 2026-08-22: `HostOwnerId`, owner별 root 집합, source 순서의 `PlannedValue`,
  충돌 없는 `ValueSlotId`를 Evaluation IR 계약에 추가했다.
- 2026-08-22: `BoundaryReason`을 제거하고 모든 host 위치를
  `HostContinuation::{Return, Discard, Compose}`로 통합했다.
- 2026-08-22: SWC 실제 연산자와 child span에서 단락·삼항·호출·멤버·생성·태그
  템플릿·suspend 평가 프로토콜을 만들었다. 문자열이나 parent-kind 추측은 사용하지
  않았다.
- 2026-08-22: 선행 TypeScript 입력을 `Value | Reference`로 구분하고, 같은 owner의
  앞선 RL 입력을 원본 source가 아닌 `ValueSlotId` 의존성으로 정규화했다.
- 2026-08-22: protocol span이 원본 origin으로 역투영되지 않으면 단계를 생략하지 않고
  `UnmappedEvaluationSpan` 내부 오류로 실패하도록 했다.
- 2026-08-22: 중간 검증으로 `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test`를 실행했다. 전체 테스트가 통과했다.
- 2026-08-22: codegen의 `.ok().unwrap_or_default()` 분석 실패 우회를 제거했다. host
  lowering이 필요한 Core 파일은 ProgramSyntax/Evaluation IR 오류를 내부 컴파일러 오류로
  처리하고, source edit만 있는 파일은 타입화된 `requires_host_lowering` 판정으로 분석을
  생략한다.
- 2026-08-22: `try`의 의미 span과 전체 source owner를 HIR/Core에서 분리했다. 선언형과
  bare 형식 모두 완전한 statement owner로 projection되며 기존 진단·anchor는 기존
  `try <expr>` span을 계속 사용한다.
- 2026-08-22: SWC `VarDeclarator`의 initializer edge를 `Initialize` continuation으로
  타입화했다. owner가 하나의 expression-arm `Decision` 값을 초기화하는 경우 공통
  value slot을 만들고, statement 제어 흐름으로 값을 할당한 뒤 원본 initializer만
  slot 참조로 바꾸는 첫 whole-owner target 전환을 적용했다.
- 2026-08-22: decision leaf의 소비 방식을 `ValueDestination::{Expression, Assign}` 공통
  continuation으로 분리했다. switch와 guarded if-chain은 같은 구조화 함수를 사용하며,
  initializer 전환은 더 이상 별도 match 출력 template을 갖지 않는다.
- 2026-08-22: SWC가 수집한 전체 TypeScript identifier 집합을 Evaluation IR에 전달하고
  생성 value slot을 충돌 없이 할당했다. source-preserving rope는 source chunk 경계가
  아닌 임의의 원본 byte 위치에 owner prelude를 삽입할 수 있도록 event sweep으로
  일반화했다.
- 2026-08-22: compile 출력 284건과 match guard runtime, exhaustive match typecheck,
  emit mapping 15건을 중간 검증했다. whole variable initializer의 expression-arm match는
  IIFE 없이 방출되고 기존 의미·mapping 검사가 통과했다.
- 2026-08-22: `ParentCollector`의 익명 3중 결과를 `CollectedProgramSyntax` 단계 타입으로
  바꿨다. 최종 검증으로 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`를 실행했고 전체 게이트가 통과했다.
- 2026-08-22: main 작업 트리를 `cargo install --path . --force`로 전역 재설치했다.
  enum과 variable-initializer match 예제를 설치된 `~/.cargo/bin/rlc`로 변환해 IIFE 없는
  slot/switch 출력을 확인했고, 생성된 TypeScript를 Node.js로 실행해 결과를 검증했다.
- 2026-08-22: expression arm의 자식 `Decision`이 부모 `ValueDestination`
  continuation을 상속하도록 target structuring을 재귀화했다. nested match가 더 이상
  자체 expression IIFE를 만들지 않는다.
- 2026-08-22: SWC concise-arrow body를 `ArrowReturn`으로 분류하고 expression body를
  source-preserving block body로 구조화했다. 311줄 commerce 예제의 initializer, arrow,
  nested decision에서 생성 IIFE가 0개임과 Node runtime 결과를 확인했다.
- 2026-08-22: 변경 후 compile 출력 테스트 286건을 실행해 모두 통과했다.
- 2026-08-22: 최종 중간 게이트로 `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test`를 실행했다. passthrough 56건, integration 77건, native
  typecheck 38건, emit mapping 15건을 포함한 전체 검증이 통과했다.
- 2026-08-22: `ValueContinuation = destination + wrappers` 타입을 도입했다. Decision leaf와
  Result propagation/success가 같은 continuation을 소비하며 `ResultOk` wrapper는 성공
  edge에만 합성된다.
- 2026-08-22: initializer, direct return, concise arrow, match arm 안의 `ResultRegion`을
  `do/while(false)` join 또는 직접 return 제어 흐름으로 낮췄다. await는 기존 async host에
  남고 별도 async closure를 만들지 않는다.
- 2026-08-22: Core `Sequence`를 선행 statement·최종 value·후행 source piece로 구조화했다.
  result의 최종 match는 각 leaf에서 바로 `Ok` wrapper와 parent slot을 소비한다.
- 2026-08-22: result 출력 테스트를 statement-region 계약으로 갱신하고 direct-return 및
  match-arm continuation 회귀 테스트를 추가했다. compile 테스트 288건이 통과했다.
- 2026-08-22: 모든 host return을 owner-scoped value slot의 단일 join으로 통합했다.
  match/result의 각 edge는 slot을 정의하고 원래 return statement가 한 번만 소비한다.
- 2026-08-22: join slot 소비 지점에 Core value anchor를 전파했다. IIFE 제거 뒤에도
  `match (input)`과 `n <- test()`의 기존 타입 진단 범위 및 전체 Result 문맥이 유지됨을
  native 진단 회귀 테스트 2건으로 확인했다.
- 2026-08-22: 새 join 출력에 맞춰 direct-return compile 테스트 4건을 갱신했다.
  compile 출력 테스트 288건이 통과했다.
- 2026-08-22: 최종 중간 게이트로 `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test`를 실행했다. unit 153건, compile 288건, mapping 15건,
  integration 77건, native 38건, passthrough 56건을 포함한 전체 검증이 통과했다.
- 2026-08-22: `ResultRegion`과 `Decision` projection이 내부 TypeScript island를 재귀
  투영하도록 바꿨다. 중첩 initializer/result/match/pipeline은 같은 source-backed owner의
  부모 continuation을 다시 상속한다.
- 2026-08-22: 호출·멤버·생성·배열·객체·대입·시퀀스·단항·template의 SWC 평가 child를
  공통 ordered protocol로 확장했다. 선행 getter·callee·인자·computed key를 owner slot에
  한 번만 저장하고 원래 좌우 순서로 소비한다.
- 2026-08-22: 한 HostOwner의 여러 RL value를 하나의 schedule로 합쳤다. RL source 입력은
  `ValueSlotId` 의존성으로 연결하고 동일 source capture는 owner 안에서 공유한다.
- 2026-08-22: SWC가 block arm의 함수 깊이별 `ReturnStmt`를 `HostExit`로 수집한다. target은
  return expression을 부모 continuation assignment와 labeled break로 낮추며 중첩 함수의
  return은 원문으로 보존한다.
- 2026-08-22: parameter initializer와 class initializer를 expression-only owner로
  타입화하고 hygiene된 `$rl_expr` intrinsic을 파일당 한 번만 방출했다. 일반 owner는
  계속 statement slot/CFG를 사용하며 생성 self-invoked closure/IIFE 경로를 제거했다.
- 2026-08-22: parameter의 `arguments`·함수 `length`, class field의 `this`, member getter와
  `this` binding, 선행/후행 인자 순서, block arm nested return을 Node runtime으로 검증했다.
- 2026-08-22: tagged template의 tag를 일반 call reference와 같은 `mode + receiver` 입력으로
  모델링했다. member tag는 receiver를 한 번 평가하고 bound reference로 만든 뒤 interpolation
  slot과 결합한다.
- 2026-08-22: optional call 인자의 부분 statement 승격을 실험하고 TypeScript의 분기별
  definite-assignment 상관관계가 보존되지 않음을 확인했다. operation 전체를 재구성하기 전에는
  expression-only target capability를 유지하도록 구조화 가능성 판정을 명시했다.
- 2026-08-22: Core operation의 직접 자식은 host owner가 달라도 `Nested` region으로 배치하는
  경로를 추가했다. propagation 안의 중첩 RL 값은 부모의 early-return continuation을 소비한다.
- 2026-08-22: 최종 게이트에서 unit 156건, compile 292건, mapping 15건, integration 80건,
  native 38건, passthrough 56건을 포함한 전체 테스트와 fmt/clippy가 통과했다.

- 2026-08-24: `c5f3e25` 기준으로 남은 범위를 재감사했다. 문서에만 있는 계약
  (validate_order/reference/origin/source_preservation, Effects), codegen에 남은
  이름 없는 capability fallback, host owner 기준 빈도의 부재를 확인하고 위
  "2026-08-24 감사" 절에 기록했다. 감사 중 세 건의 실제 결함(이슈 14~16)을
  `cargo run -- -p --no-banner`와 `tsgo --noEmit --strict`로 재현했다.

- 2026-08-24: 구조화된 internal compiler error 계층(`src/ice.rs`)을 추가했다.
  `LoweringStage`(실패 단계) × `Invariant`(위반한 이름 있는 계약) ×
  `LoweringSubject`(owner/Core root/slot) × source span × origin chain을 타입으로
  담고, 표시 문자열은 이 타입에서 파생된다. variant는 실제로 검증되는 계약이
  있을 때만 존재한다.
- 2026-08-24: `EvaluationContext`에 host owner 기준 사실
  `owner_reach: OwnerReach{Same, Repeated, UnmodeledConditional}`를 추가했다
  (결정 12). loop header는 `Repeated`, switch case test·구조분해 default·
  optional chain 꼬리는 `UnmodeledConditional`(protocol step이 조건을 재현하지
  못하는 edge), 삼항·논리 우변·optional call 인자는 step이 조건을 재현하므로
  `Same`이다. `ProjectedHostOwner`가 owner 선택 시점의 parent edge 위치를
  기록해 두 사실의 기준이 어긋날 수 없다.
- 2026-08-24: target capability를 Evaluation IR로 옮겼다(결정 13).
  `PlannedValue.capability: TargetCapability{StatementRegion,
  ExpressionBoundary(reason)}`가 owner 종류·owner_reach·Core statement form·
  schedule의 capture/reference 사실로 결정되고, codegen의
  `TargetRewritePlan::build`는 이 값을 읽어 statement *모양*만 고른다.
  `can_structure_value_expr`/`compose_schedule_is_structurable`는 삭제되고
  Core 모양 술어는 `CoreFile::has_statement_form`으로 Core IR이 소유한다.
- 2026-08-24: `validate_order`와 `validate_reference`를 실제 pipeline 단계로
  구현했다(`codegen::lowering_plan`이 plan 직후 실행, 실패는 `raise()`).
  validate_order는 capability 결정을 신뢰하지 않고 plan에서 독립 재검증한다:
  owner_reach, 값의 source 순서, slot 의존의 생산-후-소비, conditional region
  밖으로 나가는 capture(방출 순서 기준 첫 materialization), capture의
  tt span/상호 겹침, capture 방출 순서의 source 순서 보존.
  validate_reference는 receiver 없는 member reference, optional call 인자
  문맥의 member reference, value slot으로 강등된 reference를 거부한다.
- 2026-08-24: 평가 protocol을 owner-상대적으로 만들었다. host owner 밖의
  frame(함수 경계 너머의 바깥 표현식)은 그 owner의 평가 의무가 아니므로
  `finish()`가 owner span에 포함된 frame만 protocol로 합성한다. 콜백 안의
  initializer(`f(() => { const x = match ...; })`)가 boundary 없이 statement
  lowering된다 — 완료 기준 7의 한 사례.
- 2026-08-24: `validate_origin`을 상시 실행으로 바꿨다. `Rope::flatten`의
  `debug_assert`를 제거하고 `TargetError`를 구조화된 ICE로 변환해 release
  빌드도 동일하게 실패한다.
- 2026-08-24: `validate_source_preservation`을 target 단계로 구현했다.
  `SourcePreservation{owned, relocated, rewritten}`을 Core IR과 plan에서
  계산한다 — owned는 Core `Opaque`/template raw의 pass-through 집합,
  relocated는 schedule capture와 prelude로 hoist되는 값 span, rewritten은
  block arm `HostExit`의 return 프레임. printer 직전에 pass-through 바이트의
  정확히-한-번·순서 보존·비공백 누락 없음을 target piece로 검증한다. 출력
  문자열은 읽지 않는다.
- 2026-08-24: 세 결함의 회귀를 고정했다. capability 단위 테스트 9건과
  validator 위반-IR 테스트 6건(evaluation_ir), 위반-target 테스트 7건(rope),
  출력 회귀 7건(compile.rs), 런타임 회귀 3건(integration.rs — 반복 평가,
  단락 시 인자 미평가, 좌우 순서). 전체 게이트(fmt/clippy/전 스위트 13개,
  tsgo 포함)가 통과했다.

- 2026-08-24: 조건부 operation의 whole-owner lowering을 구현했다(결정 17).
  ProgramSyntax가 조건 연산의 구조 사실(`ConditionalFacts` — 활성 branch,
  건너뛰는 branch, optional call의 전체 인자 목록·spread·type args)을 SWC
  AST에서 계산하고, Evaluation IR이 같은 parent를 공유하는 단일-step 조건
  값들을 `PlannedConditionalOperation`(LogicalAnd/Or/Nullish/Ternary/
  OptionalCall)으로 묶는다. target은 연산의 parent span 전체를 result slot으로
  대체하고, region이 조건/callee를 한 번 평가한 뒤 모든 경로에서 result를
  대입한다 — optional call의 인자는 nullish 검사 안에서만 평가되고, member
  callee는 `callee.call(receiver, ...)`로 this를 보존한다(tsc downlevel과
  동일한 형태). 구조화 불가능한 조건 연산(중첩 조건, 사이에 낀 capture,
  member callee + 명시적 type args)은
  `ExpressionBoundary(ConditionalOperationNotStructurable)`로 남는다.
- 2026-08-24: 호출·생성 인자의 protocol 위치를 `ExprOrSpread.span()`에서
  인자 표현식 span + spread 사실로 바꿨다. spread 인자의 capture가
  `(...xs)`라는 잘못된 TypeScript 대신 표현식만 capture하고 `...`는
  호출 위치에 남는다.
- 2026-08-24: 이 전환으로 감사에서 확인된 계약 7 위반 네 건(&&/||/??/삼항의
  `T | undefined` 새 타입 오류), optional call 선행 인자의 무조건 평가,
  spread capture 파싱 오류가 모두 해소됐다. tsgo 타입체크 통과를
  integration 테스트(`conditional_operations_keep_their_types_without_undefined`)
  로, 단락 시 인자 미평가와 this·검사 순서를 runtime trace 테스트 2건으로,
  출력 형태를 compile 테스트 4건으로 고정했다. 기존 snapshot 1건
  (member optional call의 boundary)을 새 계약(whole operation)으로 갱신했다.
  전체 게이트 13개 스위트가 통과했다.

- 2026-08-24: Effects 모델을 구현했다(결정 18). ProgramSyntax가 protocol
  input마다 보수적 `Effects`를 계산하고, resolve_schedule이 inert한 Value
  입력을 `PlannedEvaluationInput::Stable`로 계획해 slot·capture를 생략한다.
  optional call region의 inert 선행 인자는 재구성된 호출에 인라인되고, inert
  조건은 branch에서 재평가된다 — 모두 관측 불가능성이 증명된 경우만이다.
  `g(1, match …, 2)`가 리터럴 capture 없이, `g(eff(), match …)`는 capture와
  함께 방출됨을 단위·출력 테스트로 고정했다. 전체 게이트 통과.

- 2026-08-24: mutation 검증(7.3)을 수행했다. 작업 코드를 커밋한 상태에서 각
  규칙을 의도적으로 깨고 테스트가 실제로 실패함을 확인한 뒤 `git restore`로
  복귀했다(사용자 미커밋 변경 없음 확인). 결과:

  | mutation | 기대 검출 | 결과 |
  |---|---|---|
  | M1: `owner_reach`가 항상 `Same` | capability 단위 테스트 + loop 출력 회귀 | 둘 다 실패(검출) — validator ICE `RepetitionRegionLeft`가 컴파일도 중단 |
  | M2: validate_order의 repetition 검사 제거 | 위반-plan 단위 테스트 | 실패(검출) |
  | M3: 논리 연산 region의 else 대입 제거 | tsgo 타입 테스트 + 단락 runtime | 둘 다 실패(검출) |
  | M4: optional call 첫 인자를 검사 밖에서 평가 | this·단락 runtime trace | 실패(검출) |
  | M5: `is_inert`가 항상 true | capture 출력 테스트 | 실패(검출) |
  | M5b: 〃 | **처음에는 runtime 순서 테스트가 없어 생존** → `eager_arguments_keep_left_to_right_order_at_runtime` 추가 후 실패(검출) |
  | M6: preservation 중복 검사 제거 | 위반-target 단위 테스트 | 실패(검출) |
  | M7: validate_order의 ordinal 검사 제거 | **swap 테스트가 slot-읽기 검사로도 통과해 생존** → slot 의존을 제거하고 ordinal만 남기는 테스트로 강화 후 실패(검출) |

  생존한 mutation 두 건(M5b·M7)은 테스트 공백의 증거였고, 각각 runtime 순서
  trace 테스트와 격리된 ordinal 위반 테스트를 추가해 닫았다. 명령:
  `cargo test --lib <validator test>` / `cargo test --test integration <trace test>`.

- 2026-08-24: 값 region의 exit target을 하나로 정리했다(결정 19).
  `HostExit.captured_break`를 ProgramSyntax가 계산하고, codegen은 그 사실이
  참인 exit이 있을 때만 region label을 방출한다. `match (e) { A(v) => { use(v);
  return v; }, B => 0 }`는 이제 `$tt_y_…: { … }` 없이 dispatch 자체로 나가고,
  arm body가 `for` 안에서 `return`하는 경우에만 label이 남는다. 출력 테스트
  2건을 새 계약으로 갱신하고 label 유지 케이스 1건, runtime 회귀 2건을
  추가했다. mutation M8(captured_break를 항상 false)·M9(항상 true) 모두
  테스트가 실패함을 확인했다.
- 2026-08-24: `docs/ai/tt.md`의 match 표현식 항목을 갱신했다 — 조건부 연산은
  operation 전체가 하나의 region으로 낮아지고, expression boundary는 statement가
  도달할 수 없는 owner(매개변수 기본값, class field, loop header, switch case
  test, 구조분해 default, 구조화 불가능한 조건 연산)에서만 쓰인다는 계약.

## 이슈 및 해결

### 이슈 1: placeholder 내부 가짜 statement가 owner로 선택됨

- **증상**: statement/item RL 구문이 포함된 전체 surface 테스트에서
  `MissingOverlay`가 발생했다.
- **원인**: placeholder가 만든 내부 SWC statement span은 원본 source와 선형
  대응하지 않는데 가장 가까운 owner 하나만 저장했다.
- **해결**: 전체 owner stack을 보존하고 origin mapping이 성립하는 가장 안쪽
  source-backed owner를 선택하도록 일반화했다.

### 이슈 2: Evaluation IR 생성 실패가 legacy emitter로 조용히 우회됨

- **증상**: ProgramSyntax 또는 Evaluation IR 오류가 빈 `LoweringPlan`으로 바뀌어 기존
  emitter가 계속 실행됐다.
- **원인**: shadow 단계에서 사용하던 `.ok().unwrap_or_default()` 연결이 whole-owner
  전환 뒤에도 남아 있었다.
- **해결**: host lowering 필요 여부를 Core 구조에서 판정하고, 필요한 파일의 분석 실패는
  내부 컴파일러 오류로 중단하도록 변경했다. RL owner가 없는 파일만 분석하지 않는다.

### 이슈 3: 선언형 try의 의미 span만 투영되어 TS owner가 불완전해짐

- **증상**: `const n = try value;`가 projection에서 `($rl_syntax_expr) return n;`처럼
  선언 prefix를 잃었고 SWC가 다음 statement에서 parse error를 냈다.
- **원인**: 진단용 `try <expr>` span을 전체 TypeScript source owner로도 사용했다.
- **해결**: AST→HIR→Core에 별도 `TryOwner` identity를 전달했다. projection은 전체
  statement owner를 사용하고 진단·mapping은 기존 의미 span을 유지한다.

### 이슈 4: owner prelude가 HIR source chunk 경계에서만 삽입됨

- **증상**: enum이나 선행 공백 뒤의 variable initializer를 전환하면 value slot 선언이
  출력되지 않고 rewritten initializer만 남았다.
- **원인**: owner 시작 byte가 rope의 `Source` piece 시작과 같다는 가정을 사용했다.
  HIR source piece는 여러 TypeScript owner를 포함할 수 있으므로 그 가정이 성립하지 않았다.
- **해결**: source insertion과 exclusion을 원본 byte 위치의 event로 합성했다. 하나의
  source piece 내부에서도 cursor를 분할해 prelude를 정확한 owner 앞에 삽입한다.

### 이슈 5: guarded assignment leaf의 join이 false guard에도 실행됨

- **증상**: guarded if-chain에서 guard가 거짓이어도 뒤따르는 무조건 `break`가 실행되어
  다음 arm으로 fallthrough하지 못했다.
- **원인**: leaf의 assignment만 guard body에 넣고 assignment continuation의 join을
  guard 밖에 출력했다.
- **해결**: guarded assignment의 `assignment + break`를 하나의 block으로 묶었다.
  guard가 참인 경로만 join하고 거짓 경로는 다음 arm을 계속 검사한다.

### 이슈 6: collector 결과의 익명 튜플이 단계 계약을 숨김

- **증상**: `cargo clippy --all-targets -- -D warnings`가 `type_complexity`로 실패했다.
- **원인**: overlay, owner, occupied identifier를 익명 3중 튜플로 반환했다.
- **해결**: 세 결과를 이름 있는 `CollectedProgramSyntax`로 묶어 수집 단계의 출력 계약을
  명시했다.

### 이슈 7: arrow replacement가 하나의 source piece 안에 있다고 가정함

- **증상**: `ArrowReturn` 분류와 rewrite plan은 만들어졌지만 출력은 기존 expression
  IIFE로 남았다.
- **원인**: HIR source preservation은 arrow prefix, RL expression, suffix를 별도 piece로
  유지하는데 owner source piece 하나가 RL span 전체를 포함한다고 가정했다.
- **해결**: owner 문자열을 다시 자르지 않고 Core expression을 출력하는 `emit_expr`
  경계에서 `ArrowReturnRewrite`를 소비했다. 기존 source piece 분할과 mapping을 유지한다.

### 이슈 8: 구조화된 result 내부의 ordinary statement owner가 projection에서 가려짐

- **증상**: 바깥 result는 statement region으로 전환되지만, 그 ordinary statement의
  initializer에 있는 중첩 result는 기존 expression path에 남는다.
- **원인**: ProgramSyntax projection이 바깥 `ResultRegion` 전체를 하나의 placeholder로
  바꾸므로 내부 TypeScript statement의 SWC parent path가 만들어지지 않는다.
- **해결**: `ResultRegion`과 `Decision`을 분석용 synthetic call region으로 projection하고
  ordinary statement·arm body는 원본 source segment로 재귀 투영했다. 바깥 marker는 실제
  host continuation을 유지하고, 안쪽 RL value는 같은 source-backed owner면 부모 region을
  상속한다.

### 이슈 9: 분기별 direct return이 기존 타입 진단 경계를 분해함

- **증상**: result IIFE를 제거한 뒤 오류 위치는 `<-` binding에 남았지만 진단이 전체
  `Result<number, InputError>` 불일치 대신 성공 payload의 `number/string` 불일치로
  축소되었다. 단일 join 도입 직후에는 마지막 slot 참조가 source anchor 없이 nearest
  위치로 보고되었다.
- **원인**: 하나의 source value가 여러 생성 return edge로 분해되었고, join slot 소비는
  합성 identifier라 원본 소유권을 자동으로 갖지 않았다.
- **해결**: Evaluation IR이 return value에도 slot을 할당하고 target이 모든 edge를 그
  slot로 합류시킨 뒤 원본 return에서 한 번 소비하도록 했다. slot 참조는 Decision이면
  Match anchor, ResultRegion이면 마지막 ResultBind anchor, Sequence이면 최종 value anchor를
  재귀 상속한다. 기존 두 native 진단 회귀 테스트가 수정 없이 통과했다.

### 이슈 10: decision island의 synthetic 연산이 거짓 평가 protocol을 만듦

- **증상**: arm 내부 중첩 RL과 `await`를 projection한 뒤 `UnmappedEvaluationSpan` 또는
  “await isn't allowed in non-async function” 내부 오류가 발생했다.
- **원인**: 분석용 `void` unary와 non-async synthetic arrow가 실제 source 평가 parent처럼
  수집되었다.
- **해결**: synthetic decision arrow가 Core의 async 사실을 상속하고, child는 별도 unary
  연산 없이 expression statement로 투영했다. marker call 자체는 protocol frame에서
  제외한다.

### 이슈 11: 같은 owner의 두 RL value가 서로의 source를 다시 출력함

- **증상**: constructor callee와 argument가 모두 match이면 빈 scrutinee와 원문 match가
  섞인 잘못된 TypeScript가 생성되었다.
- **원인**: owner당 값 하나를 가정했고 parenthesized callee span을 RL slot이 아닌 일반
  source input으로 분류했다.
- **해결**: transparent TS wrapper를 벗긴 reference value span을 protocol에 기록하고,
  owner의 모든 `PlannedValue`를 하나의 schedule로 합쳤다. source slot capture도 이름 있는
  `PlannedSourceSlot`으로 공유한다.

### 이슈 12: optional call 인자만 승격하면 TypeScript 타입이 달라짐

- **증상**: optional member call의 match 인자를 statement slot으로 승격하면 런타임 분기는
  맞지만 tsc가 인자 slot을 `T | undefined`로 추론했다.
- **원인**: 원래 구문은 callee가 nullish가 아닐 때만 인자를 평가한다. 부분 치환 출력에서는
  callee 조건과 이후 optional call 사이의 definite-assignment 관계를 TypeScript가 표현하지
  못한다.
- **해결**: `Conditional(OptionalCallArgument)`와 `MemberReference`의 조합은 부분 Compose
  대상에서 제외했다. 향후 conditional operation 전체가 result slot을 생산하는 target
  primitive로 모델링될 때만 statement CFG로 승격한다. direct optional reference의 조건은
  truthiness가 아니라 `!= null`로 기록했다.

### 이슈 13: opaque wrapper를 단순 sequence continuation으로 오인함

- **증상**: `try wrap(match (...))`가 `wrap({ switch ... })` 형태의 파싱 불가능한 TypeScript로
  방출되었다.
- **원인**: `Sequence`의 마지막 RL 값만 구조화 가능하면 주변 `Opaque` 조각의 내용과 무관하게
  부모 continuation을 직접 전달했다. 함수 호출의 prefix/suffix도 trivia처럼 취급되었다.
- **해결**: propagation 입력에서 주변 `Opaque` 조각이 실제로 공백뿐일 때만 continuation을
  직접 인라인한다. 실행 구문이 남은 sequence는 expression target으로 방출해 wrapper의 평가
  구조와 기존 진단 anchor를 보존한다.

### 이슈 14: loop header의 tt 값이 loop 밖에서 한 번만 평가됨

- **증상**: 다음 입력이 무한 루프가 되는 TypeScript로 컴파일된다.

  ```ts
  enum E { A, B }
  let n = 0;
  function next(): E { n = n + 1; return n < 3 ? E.A : E.B; }
  while (id(match (next()) { A => 1, B => 0 })) { console.log("tick"); }
  ```

  생성 코드는 `let $tt_v0; … { const $tt_m = next(); switch … }`를 `while` **앞**에
  놓고 `while ($tt_v1($tt_v0))`만 남긴다. `next()`가 반복마다 호출되지 않는다.
  `for` test/update, `do … while` test에서도 동일하게 재현했다.
- **원인**: capability 판정이 쓰는 `EvaluationContext.frequency`는 함수 기준
  빈도라 loop 머리를 "Repeated"로 보지만, compose 조건은
  `!steps.is_empty() || frequency == Once`여서 schedule이 비어 있지 않으면
  빈도를 아예 보지 않는다. 그리고 빈도를 봤더라도 그 빈도는 prelude가 실제로
  삽입되는 `HostOwner` 기준이 아니다 (감사 C).
- **해결**: 결정 12의 `host_frequency`를 도입하고, 결정 13의 capability가
  `RepeatedInOwner`로 statement lowering을 거부한다. `validate_order`가 계획을
  다시 검사한다.

### 이슈 15: conditional region 안의 capture가 region 밖에서 읽혀 컴파일되지 않음

- **증상**:

  ```ts
  enum E { A(v: number), B }
  export const short = flag && id(match (e) { A(v) => v, B => 0 });
  ```

  → `sc2.ts(25,32): error TS2552: Cannot find name '$tt_v1'.`
  `const $tt_v1 = (id);`가 `if ($tt_v2) { … }` 안에 선언되는데
  `export const short = $tt_v2 && $tt_v1($tt_v0);`는 블록 밖에서 읽는다.
  (`tsgo --noEmit --strict`로 확인.)
- **원인**: schedule step이 안쪽 frame부터 바깥쪽 frame 순으로 action을 감싸므로,
  바깥 conditional step이 안쪽 frame의 prefix 전체를 자기 분기 안으로 넣는다.
  conditional이 소유하는 실행 영역과 host 표현식이 사는 영역이 어긋난다.
- **해결**: 결정 14. `validate_order`가 conditional 깊이 1 이상에서 새 source
  capture가 생기면 계획을 거부한다.

### 이슈 16: capture span이 tt 값 span과 겹쳐 원본이 중복·누락됨

- **증상**:

  ```ts
  g(a && match (e) { A(v) => v, B => 0 }, match (e) { A(v) => v, B => 1 });
  ```

  → `generated TypeScript failed to parse: Expression expected.`
  `--no-verify`로 보면 `const $tt_m = ;`, `$tt_v0 = ;`처럼 scrutinee와 arm 값이
  사라지고, `const $tt_v4 = (a && match (e) { A(v) => v, B => 0 });`처럼 tt 원문이
  그대로 복사되며, 마지막 호출은 `$tt_v3($tt_v2$tt_v0, $tt_v1)`로 붙는다.
- **원인**: 두 번째 값의 schedule은 "앞선 인자"를 입력으로 갖는데, 그 인자 span이
  첫 번째 tt 값을 포함한다. `source_replacements`가 서로 겹치는 구간을 만들고
  `source_range_rope`의 cursor가 겹친 구간을 건너뛰면서 원본 바이트를 잃는다.
- **해결**: 결정 15. 계획 단계에서 겹치는 capture를 거부하고,
  `validate_source_preservation`이 target에서 중복·누락·겹침을 다시 검사한다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` — 813건 통과 (unit 197, cli 32, compile 323,
  emit-map 16, engine-cache 3, integration 94, native 39, passthrough 57,
  resolve 11, sidecar 8, stdlib 5 + doc 25 등 13개 스위트 전부 ok, 실패 0).
  기준선(c5f3e25) 764건에서 49건 증가.
  tsgo는 `@typescript/native-preview` 7.0.0-dev를 설치해
  `TTC_TSGO_API=…/dist/api/sync/api.js`로 연결해 실행했다.

### mutation 검증 결과

| mutation | 기대 검출 | 결과 |
|---|---|---|
| M1 `owner_reach`가 항상 `Same` | capability 단위 + loop 출력 회귀 | 실패(검출) |
| M2 validate_order의 repetition 검사 제거 | 위반-plan 단위 | 실패(검출) |
| M3 논리 연산 region의 else 대입 제거 | tsgo 타입 + 단락 runtime | 실패(검출) |
| M4 optional call 첫 인자를 검사 밖에서 평가 | this·단락 runtime trace | 실패(검출) |
| M5 `is_inert`가 항상 true | capture 출력 | 실패(검출) |
| M5b 〃 | runtime 인자 순서 | 최초 생존 → 테스트 추가 후 실패(검출) |
| M6 preservation 중복 검사 제거 | 위반-target 단위 | 실패(검출) |
| M7 validate_order의 ordinal 검사 제거 | 위반-plan 단위 | 최초 생존 → 테스트 격리 후 실패(검출) |
| M8 `captured_break`가 항상 false | loop 안 exit runtime + label 출력 | 실패(검출) |
| M9 `captured_break`가 항상 true | 중복 label 출력 | 실패(검출) |

mutation은 모두 작업 코드를 커밋한 뒤 수행하고 `git restore`로 복귀했다.
생존한 두 건은 테스트 공백의 증거였고, 각각 회귀 테스트를 추가해 닫았다.

## 완료 기준 점검

| # | 기준 | 상태 | 근거 |
|---|---|---|---|
| 1 | `validate_order` 실행 | ✅ | `codegen::lowering_plan`이 plan 직후 호출, 실패 시 `raise()` |
| 2 | `validate_reference` 실행 | ✅ | 같은 자리 |
| 3 | `validate_origin` 실행 | ✅ | `Rope::flatten`의 `debug_assert` 제거, release도 상시 실행 |
| 4 | `validate_source_preservation` 실행 | ✅ | printer 직전 target 단계 |
| 5 | 구조화된 internal compiler error | ✅ | `src/ice.rs` — stage × invariant × subject × span × origin |
| 6 | optional operation을 전체 CFG로 표현 | ✅ | `PlannedConditionalOperation` (결정 17) |
| 7 | statement-lowerable에 불필요한 boundary 없음 | ✅ | 대표 코퍼스에서 `$tt_expr` 0개 |
| 8 | 진짜 expression-only만 명시적 capability | ✅ | `ExpressionBoundaryReason` 6종, 이유 없는 경로 없음 |
| 9 | Effects 구현·최적화에만 사용 | ✅ | 결정 18 — 소비처는 capture 생략 한 곳 |
| 10 | slot·helper·capture 제거를 증명으로 | ✅ | capture는 Effects, region label은 `captured_break` use 분석. 증명 없는 제거는 하지 않음 (join slot은 결정 8이 타입 보존 목적으로 유지) |
| 11 | TASK-198 레이아웃·괄호 유지 | ✅ | 해당 테스트 통과 |
| 12 | TASK-199 `completes` 유지 | ✅ | 해당 테스트 통과 |
| 13 | TS/TSX passthrough 바이트 유지 | ✅ | passthrough 57건 + validator가 상시 강제 |
| 14 | 기존 진단·위치 유지 | ✅ | native 39 + emit-map 16 |
| 15 | runtime trace 보존 | ✅ | integration 94 (평가 순서·횟수·this·단락·throw) |
| 16 | SWC output verification | ✅ | 전 스위트에서 `verify_output` 활성 |
| 17 | tsgo 타입 검사 | ✅ | native 39 + 신규 타입 테스트 |
| 18 | mutation에서 테스트 실패 | ✅ | M1~M9, 아래 표 |
| 19 | `cargo fmt --check` | ✅ | |
| 20 | `cargo clippy -D warnings` | ✅ | |
| 21 | `TTC_REQUIRE_TSGO=1 cargo test` | ✅ | 813건 |
| 22 | 결정·대안·조사·원인·mutation 기록 | ✅ | 이 문서 |
| 23 | INDEX 갱신 | ✅ | 완료 처리 |
| 24 | 사용자 문서 갱신 | ✅ | `docs/ai/tt.md` match 표현식 계약 |

## 결과

완료. TASK-160이 목표한 "SWC whole-owner 기반 최적 lowering"의 남은 아키텍처를
모두 세웠다.

- 감사에서 재현한 결함 6종을 구조적으로 해소했다: loop header 값의 loop 밖
  hoist, conditional region 밖으로 새는 capture(컴파일 불가 TypeScript),
  tt 값과 겹치는 capture(파싱 불가 TypeScript), `&&`/`||`/`??`/삼항의
  `T | undefined` 타입 변형, optional call 선행 인자의 무조건 평가,
  spread 인자 capture의 `(...xs)` 파싱 오류.
- 계약이 문서에서 코드로 내려왔다: 네 validator가 실제 파이프라인 단계로
  돌고, 실패는 이름 있는 불변식을 가진 internal compiler error다.
- 의미 판단이 codegen에서 Evaluation IR로 옮겨졌다. codegen은 capability를
  읽어 statement *모양*만 고른다.

### 남은 범위 (별도 태스크)

- **일반 compile 출력용 표준 source map** — 지시 §9대로 TASK-160에 섞지 않고
  후속 태스크로 진행한다. 기반(Rope/TargetPiece/SourceOrigin/EmitMapping/
  EmitAnchor/`SourcePreservation`의 owned·relocated·rewritten)은 이미 있다.
- §6의 나머지 최적화 후보(불필요한 receiver temporary, 직접 호출로 대체 가능한
  `$tt_ap`)는 증명 수단이 갖춰졌으나 이번 범위에서 적용하지 않았다. join slot
  제거는 결정 8(전체 contextual type 보존)과 상충하므로 의도적으로 하지 않는다.
