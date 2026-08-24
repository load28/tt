# TASK-199: block arm의 도달 불가능한 폴스루 제거

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: (아래 작업 내역의 커밋)

## 목적

블록 본문 arm은 값을 내지 않고 끝날 수 있으므로(`=> { log(); }`), 로워링이
본문 뒤에 폴스루를 쓴다:

```ts
if ($tt_m.kind === "Rect") { const { w, h } = $tt_m; { const t = `${w}x${h}`;
      $tt_v0 = t; break $tt_y_$tt_v0;
          $tt_v0 = undefined;      // ← 위에서 이미 나갔다
          break $tt_b;             // ← 여기도 도달 불가
} }
```

본문이 항상 나가는 경우(모든 경로가 `return`/`throw`) 이 두 줄은 절대 실행되지
않는다. 읽는 사람은 "여기로 오는 경로가 있나?"를 매번 따져야 하고, 답은 없다.
TASK-198이 레이아웃을 고치면서 이 죽은 코드가 더 눈에 띄게 됐다(그 태스크의
후속 후보로 기록).

## 범위

- 포함: arm 본문의 "제어가 끝에 도달할 수 있는가"를 파서의 flow CFG로 답하고
  AST→HIR→Core IR로 옮겨 codegen이 소비, 그 사실에 따라 폴스루와 그 뒤 exit,
  그리고 쓰이지 않게 된 체인 레이블(`$tt_b`)을 방출하지 않음, 실행 회귀 테스트
- 제외: `do { … } while (false)` + 레이블 블록의 이중 중첩 단일화, 임시 이름
  체계(`$tt_y_$tt_v1`), 표현식 본문 arm(항상 값을 낸다)

## 의사결정

### 결정 1: 답은 파서가 flow CFG로 내고, 계층을 타고 내려간다

- **상황**: codegen에는 이 질문에 답할 자료가 없다 — `codegen/core.rs`는 파서
  AST와 무관하도록 설계돼 있고(모듈 문서), 본문은 이미 HIR body다.
- **검토한 대안**:
  - **A. codegen에서 본문의 마지막 문장 모양을 본다.** "마지막이 `return`이면"
    같은 판정. 정확하지도 않고(if/else 양쪽 발산, `try`/`finally`, 무한 루프)
    CLAUDE.md 불변 원칙 3이 금지하는 모양 기반 휴리스틱이다.
  - **B. `program_syntax`의 swc AST 위에서 도달성을 다시 구현한다.** 같은
    의미를 두 번 구현하게 된다 — 이미 `crate::flow`가 있다.
  - **C. 파서가 `crate::flow::program_diverges`로 답하고 그 사실을 계층으로
    옮긴다.** let-else의 "else 블록은 발산해야 한다"(TASK-172/173)와 **같은
    질문, 같은 분석기**이며, 그 경로는 이미 AST(`LetElseStmt::diverges`)로
    운반되고 있다.
- **선택과 근거**: **C**. 새 분석을 만들지 않고 기존 CFG에 질문 하나를 더
  물을 뿐이다. `parse_arm_tail`은 이미 `body_tokens`와 파싱된 `body`를 손에
  들고 있어 let-else와 문자 그대로 같은 두 줄이다. flow의 계약("발산하지
  않는다는 답은 낼 수 있어도, 없는 발산을 지어내지는 않는다")이 안전 방향을
  보장한다 — 틀리면 오늘의 죽은 코드가 남을 뿐이다.

### 결정 2: 사실은 `ArmBodyKind::Block`이 들고 다닌다

- **상황**: HIR→Core→codegen으로 옮길 자리가 필요했다.
- **검토한 대안**: `SiteArm`에 별도 `body_completes: bool` 필드 / `ArmBodyKind`
  의 `Block` 변형에 필드 추가.
- **선택과 근거**: 후자. 이 사실은 **블록 본문에만** 의미가 있다 — 표현식
  본문은 평가되면서 값을 낸다. 별도 불리언은 표현식 arm에서도 값이 존재하는
  것처럼 보이게 하고, 두 필드가 어긋날 수 있다. `Block { completes }`는 어긋날
  수 없다. Core IR의 `ArmAction::Yield { kind }`가 이미 이 타입을 나르고 있어
  중간 계층 변경이 없었다.

### 결정 3: 레이블도 쓰일 때만 방출한다

- **상황**: 폴스루를 지우자 `break $tt_b;`가 사라졌고, 그 결과 `$tt_b: {`가
  **한 번도 참조되지 않는 레이블**로 남았다 — 죽은 코드를 지우려다 죽은
  스캐폴딩을 남기는 셈이다.
- **선택과 근거**: `needs_label`(Core IR `match_kind`)의 근거를 "블록 arm이
  있는가"에서 "**끝에 도달할 수 있는** 블록 arm이 있는가"로 바꿨다. 레이블의
  존재 이유가 곧 그 조건이다 — 체인 끝으로 돌아갈 길이 필요한 arm. 바깥
  exit 레이블(`$tt_y_…`)은 그대로다: 발산하는 arm의 `return`이 그리로 나간다.

## 작업 내역

- 2026-08-24: `src/parser/matches.rs` — `parse_arm_tail`의 반환을 이름 있는
  `ArmTail` 구조체로 바꾸고(4-튜플이 5-튜플이 되는 것을 피함) 블록 본문에
  대해 `crate::flow::program_diverges`를 호출. 단일 arm과 튜플 arm 양쪽 생성
  지점에 전달.
- `src/ast.rs` — `Arm::diverges`, `TupleArm::diverges`.
- `src/hir/mod.rs` — `ArmBodyKind::Block { completes: bool }`.
- `src/hir/lower.rs` — `arm_body_kind(block, diverges)` 한 곳에서 만들고 단일·
  튜플·`if let` 세 경로가 공유. `if let` 본문은 실행되는 블록이라 이 사실을
  읽는 소비자가 없어 `completes: true`(증명하지 않은 발산을 주장하지 않음).
- `src/core_ir/lower.rs` — `needs_label`을 `completes: true`인 블록 arm으로
  한정. `src/program_syntax.rs`, `src/codegen/core.rs`의 패턴 매칭 갱신.
- `src/codegen/core.rs` — `emit_arm_action`이 `completes`일 때만 폴스루 대입과
  그 뒤 `break`를 쓴다(체인/스위치 양쪽).
- 테스트: `tests/integration.rs`에
  `a_block_arm_yields_the_same_value_whether_or_not_it_can_fall_out` —
  항상 나가는 arm / 조건부로 나가는 arm / 나가지 않는 arm 세 가지를 각각 다음
  arm이 있는 match에서 **실행**하고 값을 확인한다.
  `tests/compile.rs`의 `tuple_match_block_bodies_and_guards_use_the_label`은
  이제 레이블이 없어야 한다는 단언으로 갱신.
- 검증: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `TTC_TSGO_ROOT=… TTC_REQUIRE_TSGO=1 cargo test` — 13개 테스트 바이너리 전부
  통과.

## 이슈 및 해결

### 이슈 1: 위험한 방향의 오답을 잡는 테스트가 필요했다

- **증상**: 이 최적화의 위험은 한 방향으로만 있다 — **없는 발산을 주장**하면
  `switch`에서 다음 `case`로 흘러 들어간다. 문자열 단언은 이걸 못 잡는다.
- **원인**: 오방출된 코드가 문법적으로 멀쩡하다(출력 자가 검사 통과).
- **해결**: 세 가지 본문 모양을 실행하는 통합 테스트를 먼저 쓰고 변이로
  확인했다. `completes`를 항상 `false`로(= 항상 발산한다고 주장) 만들면
  `positive|yes|b|positive|**b**|b|**b**|b` — `undefined`여야 할 자리에 다음
  arm의 값이 나온다. 잡힌다. 반대 방향(항상 `true`)은 통과하는데, 그건 오늘의
  중복 코드가 남을 뿐 의미가 같기 때문이다 — 안전 방향이 통과하는 것이 맞다.

### 이슈 2: 작업 중 `git restore`로 미커밋 수정을 두 번 날렸다

- **증상**: 변이 테스트를 되돌리려고 `git restore <파일>`을 썼는데, 같은 파일에
  있던 이번 작업의 수정이 함께 사라졌다.
- **원인**: 변이 되돌리기와 작업 되돌리기가 같은 명령이다.
- **해결**: 작업을 먼저 커밋하고 그 위에서만 변이를 돌린다. TASK-198의 이슈 8과
  같은 원인이라 그때 정한 순서를 지키지 못한 것 — 두 번째는 즉시 알아채고
  복구했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (`TTC_TSGO_ROOT`로 typescript-go 연동, `TTC_REQUIRE_TSGO=1`)

## 결과

블록 arm이 항상 나가는 경우 로워링이 그 사실을 알고 아무것도 덧붙이지 않는다:

```ts
// 이전
case "A": { const { v } = $tt_m; if (v > 0) { $tt_v0 = v; break $tt_y_$tt_v0; }
    throw new Error("no");
      $tt_v0 = undefined;
      break; }
// 지금
case "A": { const { v } = $tt_m; if (v > 0) { $tt_v0 = v; break $tt_y_$tt_v0; }
    throw new Error("no"); }
```

끝에 도달할 수 있는 arm은 그대로 폴스루를 받는다 — 값이 `undefined`라는 사실이
코드에 남아 있어야 하는 유일한 경우다.

변경 파일: `src/ast.rs`, `src/parser/matches.rs`, `src/hir/mod.rs`,
`src/hir/lower.rs`, `src/core_ir/lower.rs`, `src/program_syntax.rs`,
`src/codegen/core.rs`, `tests/compile.rs`, `tests/integration.rs`.

남은 후속 후보: `do { … } while (false)` + 레이블 블록의 이중 중첩 단일화
(표현식 arm의 `break;`를 레이블 break로 바꿔야 한다), 임시 이름 정리.
