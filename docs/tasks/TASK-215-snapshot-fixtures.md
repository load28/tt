# TASK-215: 스냅샷 픽스처 — 방출과 진단을 통째로 고정한다

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: `e893b33`

## 목적

`tests/compile.rs`의 어서션은 `contains(` 479개 대 `assert_eq!` 175개다. 즉 방출
결과를 **부분 문자열**로만 확인한다. 그 결과 TASK-198이 정한 "방출 코드 가독성 —
레이아웃 계층과 그룹핑 규칙"을 지키는 게이트가 없다. 여분의 문장, 깨진 들여쓰기,
중복된 헬퍼는 전부 `contains("switch ($tt_m.kind)")`를 통과한다.

TASK-213이 만든 렌더된 진단도 같은 문제를 갖는다. 캐럿 한 칸, `= help:` 한 줄이
어긋나도 `contains("not exhaustive")`는 통과한다.

이 태스크는 **전체 산출물을 파일로 고정**한다. diff가 곧 리뷰 대상이 되어, 방출
품질과 진단 표현이 눈에 보이는 것이 된다 — rustc의 `tests/ui`가 하는 일이다.

## 범위

- 포함:
  - `tests/fixtures/`와 이를 걷는 러너 `tests/snapshot.rs`
  - `UPDATE_EXPECT=1`로 기대 파일 갱신
  - 방출: `expected.ts` — 컴파일 결과 전체
  - 진단: `expected.stderr`(CLI 렌더 결과 전체)와 `expected.json`(`--server`
    와이어 포맷 전체). 둘 다 고정해야 두 출구가 드리프트하지 않는다
  - 각 구문과 각 진단 계열을 덮는 초기 픽스처 집합
- 제외:
  - `tests/compile.rs`의 479개 `contains` 어서션을 일괄 이관하는 것. 그 파일의
    많은 테스트는 "이 한 가지 성질"을 검사하는 단위 테스트이고 스냅샷이 대체할
    대상이 아니다. 스냅샷은 **산출물 전체**가 계약인 곳에 쓴다.

## 의사결정

### 결정 1: 스냅샷을 인라인 문자열이 아니라 파일로 둔다

- **상황**: `expect-test`류는 기대값을 소스의 인라인 문자열에 두고 갱신한다.
  파일 픽스처와 둘 중 하나를 골라야 했다.
- **검토한 대안**:
  - A. 인라인. 입력과 기대값이 한눈에 붙어 있다. 다만 방출된 TypeScript는 수십
    줄이고, 그것이 테스트 소스 안에 이스케이프되어 들어가면 읽기가 나빠진다.
  - B. 디렉터리 픽스처. `input.tt`가 진짜 `.tt` 파일이라 에디터가 하이라이팅하고,
    `expected.ts`가 진짜 TypeScript라 읽는 그대로가 사용자가 받는 것이다.
- **선택과 근거**: B, 그리고 외부 의존성이 0개로 유지된다는 부수 효과가 있다
  (`Cargo.toml`에 `[dev-dependencies]`가 여전히 없다). rustc의 `tests/ui`가 같은
  형태다.

### 결정 2: 진단 픽스처는 CLI 텍스트와 와이어 포맷을 **둘 다** 고정한다

- **상황**: `expected.stderr`만으로도 렌더러 회귀는 잡힌다.
- **선택과 근거**: 둘 다 고정한다. TASK-213이 고친 결함이 정확히 "한 모델에서 나온
  진단이 출구마다 다른 사실을 갖는" 것이었다. 텍스트만 고정하면 `code`나 `edit`이
  에디터로 가는 경로에서 조용히 빠져도 아무도 모른다. `expected.json`은
  라이브러리에서 재구성하지 않고 **실제 `ttc --server` 프로세스**에서 받는다 —
  서버가 보내는 것을 고정해야 의미가 있기 때문이다.

### 결정 3: `tests/compile.rs`의 479개 `contains`를 일괄 이관하지 않는다

- **상황**: 목적이 "부분 문자열 어서션이 회귀를 놓친다"였으므로 전부 바꾸고 싶은
  유혹이 있다.
- **선택과 근거**: 하지 않는다. 그 파일의 다수는 "이 한 가지 성질"을 검사하는
  단위 테스트이고(예: `matches("switch ($tt_m.kind)").count() == 3`), 스냅샷으로
  바꾸면 무엇을 주장하는지 오히려 흐려진다. 스냅샷은 **산출물 전체가 계약**인
  곳에 쓴다. 두 방식은 대체재가 아니라 서로 다른 질문이다.

### 결정 4: 픽스처 무결성도 테스트한다

- **상황**: 픽스처는 조용히 썩는다 — 이름을 바꾼 케이스의 기대 파일이 남거나,
  아무도 읽지 않는 파일이 생긴다.
- **선택과 근거**: `no_fixture_file_is_stale_or_missing`이 각 디렉터리의 파일
  목록이 정확히 기대 집합과 같은지 본다. 갱신 실행 중에는 건너뛴다 — 한 바이너리의
  테스트는 동시에 돌기 때문에, 아직 파일을 쓰고 있는 다른 테스트와 경합한다
  (이슈 1).

## 작업 내역

- 2026-08-25: `tests/snapshot.rs` 러너 작성 — 픽스처를 걷고, `UPDATE_EXPECT=1`로
  기대 파일을 쓰고, 아니면 비교한 뒤 줄 단위 diff와 함께 실패한다. 의존성 없이
  diff를 직접 구현했다.
- 2026-08-25: 방출 픽스처 10개(enum/match, 블록 암과 가드, 리터럴·or 패턴, 튜플
  match, try + result 블록, let-else + if let, 파이프라인과 flow, 중첩 패턴,
  val과 통과 코드, `.ttx`의 JSX)와 진단 픽스처 7개(unknown case/field,
  exhaustiveness, 중복 암과 와일드카드, val 변형, let-else 비발산, stray pipe,
  한 파일의 여러 진단) 추가.
- 2026-08-25: 생성된 기대 파일을 읽어 검토했다. 두 자리 줄 번호에서 거터 폭이
  올바르게 넓어지고, 진단 블록이 빈 줄로 분리되며, 와이어 포맷이 `code`와
  `suggestions.edit`을 온전히 싣는 것을 눈으로 확인했다.
- 2026-08-25: `AGENTS.md`와 `CONTRIBUTING.md`에 "산출물 전체가 계약인 것은
  픽스처로" 항목을 추가했다.

## 이슈 및 해결

### 이슈 1: 갱신 실행에서 무결성 테스트가 경합한다

- **증상**: `UPDATE_EXPECT=1 cargo test --test snapshot`의 첫 실행에서
  `no_fixture_file_is_stale_or_missing`만 실패했다.
- **원인**: 한 테스트 바이너리의 테스트들은 동시에 실행된다. 무결성 검사가 아직
  기대 파일을 쓰고 있는 생성 테스트들과 경합해, 없는 파일을 "누락"으로 본다.
- **해결**: 갱신 중에는 무결성 검사를 건너뛴다. 검사할 대상은 갱신 **후**의
  디렉터리이고, 그것은 다음 평범한 실행이 본다. 기여자가 갱신 명령에서 혼란스러운
  실패를 만나지 않는 것이 이 검사보다 중요하다.

### 관찰 (버그 아님): 블록 암 본문의 들여쓰기

픽스처 `emit/match-arm-blocks-and-guards`가 보여주는 대로, 블록 암 본문은
사용자가 쓴 그대로 복사되므로 생성된 `if (...) { ... {` 뒤에서 줄이 들쭉날쭉해
보인다. 이는 계약("your own code is copied byte-for-byte and never reformatted")
그대로이고 결함이 아니다. 다만 **이제 눈에 보인다** — 픽스처의 목적이 정확히
그것이고, 가독성 개선을 다룰 태스크는 이 파일을 기준선으로 쓸 수 있다.

### 관찰 (버그 아님): 런타임 import가 파일 끝에 붙는다

`emit/pipeline-and-flow`는 `import { $tt_ap, $tt_fl } from "@tt/runtime";`가
그것을 쓰는 코드 **뒤**, 파일 맨 끝에 나오는 것을 보여준다. import는 호이스팅
되므로 유효한 TypeScript이지만, 생성된 `.ts`를 여는 사람에게는 낯설다. 역시
계약 위반은 아니고, 이제 고정되어 있으니 바꾸려는 태스크가 diff로 확인할 수 있다.

## 검증

toolchain 구성 후(`TTC_TSGO_ROOT`, `TTC_REQUIRE_TSGO=1`) 실행.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings` — 경고 0
- [x] `cargo test` — 전 스위트 통과, skip 없음. 새 `snapshot` 타깃 4건 포함
- [x] `UPDATE_EXPECT=1 cargo test --test snapshot` 후 재실행 시 diff 없음
      (생성이 결정적이라는 확인)

## 결과

### 추가된 파일

- `tests/snapshot.rs` — 러너. 픽스처 순회, 갱신, 줄 단위 diff, 무결성 검사
- `tests/fixtures/emit/` — 10개 케이스(41개 파일 중), 각 `input.tt`/`input.ttx`와
  `expected.ts`/`expected.tsx`
- `tests/fixtures/diagnostic/` — 7개 케이스, 각 `input.tt`, `expected.stderr`,
  `expected.json`
- `AGENTS.md`, `CONTRIBUTING.md` — 어떤 테스트를 어디에 쓰는지에 항목 추가

전체 픽스처 17KB. 외부 의존성 없음 — `Cargo.toml`의 `[dev-dependencies]`는 여전히
비어 있다.

### 관찰: 아직 문장 안에 남은 수정 조언

`diagnostic/stray-pipe`와 `diagnostic/let-else-not-diverging`의
`expected.stderr`에는 `= help:` 줄이 없다. 두 진단 모두 **고치는 법을 메시지 괄호
안에** 담고 있기 때문이다 ("parenthesize ternaries and arrow functions",
"end it with `return`, `throw`, ..."). TASK-213 결정 2는 그 조언이 `Suggestion`에
있어야 한다고 정했고 `unknown-case`와 `match-not-exhaustive`는 그렇게 옮겼지만,
나머지 규칙은 아직 옮기지 않았다. 지금은 규칙마다 조언의 위치가 다르다.

이 픽스처들이 그 불일치를 눈에 보이게 고정한다. 옮기는 작업은 사용자에게 보이는
문구 변경이라 별도 태스크로 다루는 것이 맞고, 그때 이 파일들의 diff가 곧 검토
대상이 된다.

### 후속

- [TASK-218](./TASK-218-suggestions-for-remaining-rules.md) — 남은 규칙들의
  수정 조언을 `Suggestion`으로 옮기기 (위 관찰)
- [TASK-219](./TASK-219-generated-code-readability.md) — 블록 암 본문 들여쓰기와
  파일 끝 런타임 import (위 관찰 두 건). 이제 기준선이 파일로 있으니 diff로
  검토할 수 있다.
- 픽스처 확장은 점진적으로. 새 구문·새 진단이 생길 때 케이스를 하나 추가하는 것이
  기본 동선이 되도록 `AGENTS.md`에 적었다.
