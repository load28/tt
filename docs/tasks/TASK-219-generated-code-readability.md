# TASK-219: 방출 코드 가독성 — 블록 암 들여쓰기와 런타임 import 위치

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: (아래 "작업 내역")

## 목적

TASK-215의 스냅샷 픽스처가 방출 결과를 파일로 고정하면서 두 가지가 눈에 보이게
됐다. 둘 다 계약 위반은 아니지만, 생성된 `.ts`를 사람이 연다는 점에서 품질
문제다.

**1. 블록 암 본문의 들여쓰기** (`tests/fixtures/emit/match-arm-blocks-and-guards`)

```ts
      if ($tt_m.kind === "Value") { const { n } = $tt_m; { const label = n.toFixed(1);
      $tt_v0 = `value ${label}`; break;
      } }
```

**2. 런타임 import가 파일 끝에 붙는다** (`tests/fixtures/emit/pipeline-and-flow`)

```ts
export const once = $tt_ap(half(4), twice).toFixed(1);
...
import { $tt_ap, $tt_fl } from "@tt/runtime";
```

## 범위

- 포함:
  - 2번(런타임 import 위치)을 먼저 다룬다.
  - 1번은 계약과 정면으로 만난다: 사용자 코드를 재포맷하지 않으면서 생성된
    래퍼가 그 코드를 더 나은 자리에 놓을 수 있는지 검토한다.
- 제외:
  - 사용자 코드의 재포맷.

## 의사결정

### 1. import 위치 — "맨 위"가 아니라 "import를 쓸 수 있는 첫 자리"

파일이 필요로 하는 헬퍼는 방출이 **끝나야** 알 수 있고, import가 놓일 자리는
그것과 무관한 질문이다. 그래서 `Rope::insert_lit_at_source(at, text)`를 만들어
방출이 끝난 뒤 위치를 정한다. 조각을 쪼개 넣어도 소스 커버리지와 순서 불변식은
그대로다(두 조각이 원래 바이트를 그대로 가리킨다).

`at`을 그냥 0으로 두면 안 된다. **directive prologue**(`"use client"`,
`"use strict"`)는 파일 맨 앞에 있을 때만 directive이므로, 그 위에 import를
쓰면 조용히 문자열 식으로 바뀌고 번들러가 작성자가 선언한 경계를 못 본다.
shebang도 같다. 그래서 `directive_prologue_end(source)` — shebang 한 줄과
directive 열을 건너뛰는 ASCII 바이트 스캐너 — 가 자리를 답한다.

directive인지 식인지는 문자열 **뒤에 오는 것**이 정한다. `;`이나 줄바꿈이면
directive, `+` 같은 것이 오면 `"a" + b`라는 식이므로 스캔을 멈춘다. 이 판단이
없으면 문자열로 시작하는 평범한 식 위에 import를 못 쓰게 된다.

라이선스 주석 같은 나머지 선두 텍스트는 import가 앞설 수 있는 평범한 텍스트라
자리에 영향을 주지 않는다.

### 2. 블록 암 — 사용자 코드를 **덜** 건드리는 것이 답이었다

계약과 충돌할 줄 알았던 항목인데, 원인이 반대였다. 래퍼는 자기 중괄호를 쓰고
본문을 그 사이에 복사하는데, 복사 전에 본문 rope를 `trim()`하고 있었다 —
작성자가 자기 `{` 뒤에 쓴 줄바꿈과 들여쓰기가 **지워지고** 있었던 것이다.
그래서 첫 문장만 래퍼 줄에 붙고 나머지는 원래 열에 남아 들쭉날쭉했다.

시도한 대안:

| 안 | 결과 |
|---|---|
| 그대로 둔다 | 첫 문장이 래퍼 줄에 붙는다 |
| `{` 뒤에 생성 break(`depth + 1`) | 첫 문장은 생성 열, 나머지는 작성자 열 — 여전히 어긋난다 |
| **앞쪽 trim을 하지 않는다** | 작성자의 모든 줄이 작성자가 쓴 열에 선다 |

세 번째를 골랐다. 이것은 재포맷의 반대다 — 지우던 바이트를 지우지 않는
것이므로 계약 1·2와 충돌하지 않고, 오히려 "your own code is copied
byte-for-byte" 쪽으로 한 걸음 더 간다. `Rope::trim_end()`를 나눠 뒤쪽만
다듬는다(닫는 `}`의 자리는 생성 레이아웃이 정해야 하므로).

블록 암 중 이 처리를 받는 것은 `if` 체인으로 낮아지는 갈래뿐이다. 다른 갈래는
본문을 한 줄에 이어 붙이므로 앞쪽 공백이 의미를 갖지 않는다.

### 3. `$tt_expr` 경계 헬퍼는 파일 끝에 그대로 둔다

같은 자리에서 방출되는 `function $tt_expr<T>(run: () => T): T`는 옮기지
않았다. 함수 선언은 호이스팅되고, 생성 코드에서 헬퍼 함수가 파일 끝에 있는
것은 낯설지 않다. import는 다르다 — `import/first` 같은 관례와 번들러의
해석이 걸린 자리이므로 위치가 실제 의미를 갖는다.

## 작업 내역

1. `src/codegen/rope.rs`: `insert_lit_at_source` 추가(최상위 조각만 대상,
   필요하면 소스 조각을 둘로 쪼갠다). `trim()`을 `trim_back()`으로 나누고
   `trim_end()`를 공개했다.
2. `src/codegen/core.rs`: `directive_prologue_end`, `skip_trivia`,
   `string_literal_end` 추가. 런타임 import를 그 자리에 삽입한다.
3. `src/codegen/core.rs`: 체인 갈래 블록 암의 본문을 `trim_end()`로 다듬고
   래퍼는 `{`만 쓴다.
4. `tests/compile.rs`: import 위치 3케이스(평범한 파일, directive, shebang,
   directive가 아닌 선두 문자열)와 블록 암 정렬 회귀 테스트.
5. 픽스처 2개 갱신, `docs/ai/tt.md` 갱신.

## 이슈 및 해결

- **증상**: `pipeline_inside_match_scrutinee_arm_and_template`에서
  `attempt to subtract with overflow`.
- **원인**: `insert_lit_at_source`에서 `(*src < at).then_some(at - src)` —
  `then_some`의 인자는 조건과 무관하게 **먼저 계산**되므로 `src > at`인 조각에서
  언더플로했다.
- **해결**: `then(|| at - src)`로 지연 계산. 조건이 거짓일 때 값을 만들지 않는
  것이 애초에 의도였다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 전체 초록
- [x] `UPDATE_EXPECT=1 cargo test --test snapshot` 후 diff 검토 — 픽스처 2개가
      정확히 의도한 두 변화만 보여준다

## 결과

```ts
import { $tt_ap, $tt_fl } from "@tt/runtime";
declare function half(n: number): number;
...
```

```ts
      if ($tt_m.kind === "Value") { const { n } = $tt_m; {
      const label = n.toFixed(1);
      $tt_v0 = `value ${label}`; break;
      } }
```

### 변경 파일

- `src/codegen/rope.rs`, `src/codegen/core.rs`
- `tests/compile.rs`
- `tests/fixtures/emit/pipeline-and-flow/expected.ts`
- `tests/fixtures/emit/match-arm-blocks-and-guards/expected.ts`
- `docs/ai/tt.md`
- `docs/tasks/INDEX.md`
