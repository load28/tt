# TASK-194: projection parse 실패의 원인 분류 — 사용자 TypeScript 오류를 진단으로

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: (아래 작업 내역의 커밋)

## 목적

클레임된 tt 구문(match arm 본문, `result` 블록 등) **안의 원문 TypeScript가
문법적으로 잘못됐을 때** ttc가 위치 있는 진단 대신 panic했다:

```
$ ttc --check v5.tt        # const x = match (s) { A(v) => { const q = ; return q; }, _ => 0 };
thread 'main' panicked at src/codegen/core.rs:37:21:
internal compiler error: TypeScript owner construction failed: Parse { message: "Expression expected", ... }
```

에러 계층 분리 계약("모든 tt 수준 에러는 `파일:행:열`과 함께 ttc가 직접 보고")과
codegen의 무오류 방출 계약을 동시에 어긴다. 통과 영역의 같은 오류는 정상적으로
`v2.tt:2:7: generated TypeScript failed to parse ...`로 보고되므로, 클레임된 구문
경로만 진단 채널이 없는 상태였다.

## 범위

- 포함: projection parse 실패의 원인 분류(복사된 원문 vs 컴파일러 placeholder),
  host lowering plan 구성을 emission 밖으로 분리, 사용자 원인 실패의 진단화와
  emission 차단, 무오류 소비자(`emit_mapped`)의 강등 경로, 세 계층 회귀 테스트,
  설계 문서(program-lowering.md §11.1)·AI 문서 갱신
- 제외: 기존 output self-check(`verify_output`)의 문안·동작 변경, 진짜 컴파일러
  불변식 위반의 ICE 정책 변경(그대로 panic 유지), verify 경로의 tt-lookalike
  휴리스틱 개선

## 의사결정

### 결정 1: projection parse 실패는 "멈춘 바이트의 종류"로 원인을 분류한다

- **상황**: `ProgramSyntax::build`의 parse 실패를 전부 internal compiler error로
  취급하고 있었다. 그러나 projection은 "원문 + tt 값 placeholder"이므로 그 parse는
  컴파일러 불변식이 아니라 **입력에 대한 전제**이고, 사용자가 쓴 TypeScript 때문에도
  실패한다. 원인을 구분할 규칙이 필요했다.
- **검토한 대안**:
  - (A) 메시지 문자열이나 입력 모양으로 추정 — CLAUDE.md 3번(문자열 휴리스틱 금지)
    위반이고, 같은 메시지가 두 원인 모두에서 나온다.
  - (B) parse 실패를 전부 사용자 진단으로 강등 — 컴파일러가 만든 placeholder가
    깨져도 조용히 사용자 탓이 되어 ttc 버그가 숨는다.
  - (C) parse가 멈춘 projected byte가 **어느 segment에 속하는지** 조회 — projection은
    이미 `ProjectionSourceSegment { projected, source, kind: Copied | Placeholder }`
    표를 가지고 있고, 이 표는 span 매핑의 단일 소유자다.
- **선택과 근거**: (C). 원인이 추측이 아니라 조회로 결정되고, 구조적으로 같은 모든
  입력(어떤 구문 안이든, 어떤 TS 오류든)에 같은 규칙이 적용된다. `Copied`면
  `SourceNotTypeScript { message, source }`(입력 사실, 보고할 소스 바이트 포함),
  placeholder면 기존 `Parse`(= ICE)다. swc 오류 span의 시작 바이트를 사용하고,
  `<eof>` 기대처럼 끝을 한 바이트 넘기는 경우만 마지막 바이트로 clamp한다.

### 결정 2: 실패할 수 있는 절반(plan 구성)을 emission 밖으로 옮긴다

- **상황**: `emit_with_map`이 내부에서 ProgramSyntax·Evaluation IR·LoweringPlan을
  만들고 실패 시 panic했다. emission은 계약상 무오류인데, 실패 가능한 단계가 그
  안에 들어 있어서 진단을 보고할 계층이 없었다.
- **검토한 대안**:
  - (A) `emit_with_map`을 `Result`로 바꾼다 — 무오류 방출 계약과 `emit_mapped`의
    infallible 공개 계약이 함께 흔들린다.
  - (B) codegen에서 panic을 catch해 진단으로 바꾼다 — TASK-180이 이미 기각한 방식
    (계층 불변식을 숨긴다).
  - (C) `codegen::lowering_plan(...) -> Result<LoweringPlan, SourceNotTypeScript>`를
    분리하고 `emit_with_map`은 완성된 plan을 받는다.
- **선택과 근거**: (C). "새 검사 = 검사 단계, 방출 = 무오류"라는 파이프라인 규범을
  코드 형태로 되돌린다. 진단을 소유한 단계(`compile_mapped`/`compile_report`)가 plan을
  만들고 실패를 보고하며, emission은 다시 실패할 수 없는 함수가 된다. 부수 효과로
  `emit_with_map`에서 `source_kind` 인자가 사라졌다 — TypeScript 표면 종류는 이제
  projection 단계만 안다.

### 결정 3: 무오류 소비자는 host lowering을 생략한 출력으로 강등한다

- **상황**: `emit_mapped`(에디터 가상 TS 문서, `--emit-map`)는 공개 문서에
  infallible이라고 명시돼 있는데 같은 panic 경로를 타고 있었다. 편집 중 버퍼는
  일상적으로 TypeScript가 아니다.
- **검토한 대안**: 원문을 그대로 돌려준다(매핑·anchor가 거짓이 된다) / 빈 출력을
  준다(에디터가 문서를 잃는다) / plan 없이 방출한다.
- **선택과 근거**: `lowering_plan(...).unwrap_or_default()`. host lowering이 필요 없는
  파일이 이미 받는 것과 정확히 같은 경로이고, 보고는 `compile`/`compile_report`가
  계속 소유한다.

### 결정 4: 새 진단 코드 `source-not-typescript`를 만든다 (VerifyFailed 재사용 아님)

- **상황**: 기존 `VerifyFailed`("생성된 출력이 자가 검사를 통과하지 못함")를 재사용할지
  결정해야 했다.
- **검토한 대안**: 재사용 — 코드가 하나로 유지되지만, 대상(원문 vs 출력)·행동 가능성
  (항상 사용자 원인 vs ttc 버그일 수도 있음)·우회 가능성(`--no-verify`가 통하는가)이
  모두 다르다. 코드는 규칙 단위이므로 다른 규칙이다.
- **선택과 근거**: 새 코드 `DiagnosticCode::SourceNotTypeScript`를 추가하고
  `blocks_projection()`에 포함한다(이 파일에는 projection이 없다는 사실을 소비자가
  코드만 보고 알 수 있다). enum은 `#[non_exhaustive]`라 추가는 호환된다.

### 결정 5: 이 경계에서는 tt-lookalike 추정을 하지 않는다

- **상황**: 첫 구현은 `verify::at_source`의 문안(`\`match\` here did not parse as a tt
  \`match\` ...`)을 재사용했다. 그 결과 `const x = match (s) { A(v) => { const q = ; ... } }`가
  "match가 tt match로 파싱되지 않았다"고 보고했다 — **정상적으로 클레임된** match를
  범인으로 지목한 것이다. 원인은 그 문안을 고르는 `tt_construct_at`이 오류가 난
  **줄 전체에서 키워드를 문자열로 찾는** 휴리스틱이기 때문이다.
- **검토한 대안**: 휴리스틱을 "키워드 바이트 위에서 멈췄을 때만"으로 좁힌다(여전히
  추정이고, swc는 보통 키워드 다음 토큰에서 멈춘다) / 이 경계에서 사용하지 않는다.
- **선택과 근거**: 사용하지 않는다. projection 경계에서는 클레임된 구문이 이미
  알려져 있으므로, 같은 줄의 키워드는 "잘 파싱된 구문"일 가능성이 높다. 증명할 수
  없는 원인 주장은 없는 편이 낫고, 클레임에 실패한 lookalike는 이미 자기 진단
  (`stray-*`, `malformed-*`)을 가진다. 메시지는 projection이 증명하는 것만 말한다:
  어느 바이트에서 멈췄고, 왜 컴파일이 끝나는지. `verify` 경로의 휴리스틱은 이번
  범위 밖이라 그대로 뒀다(남은 부채).

## 작업 내역

- 2026-08-24: `ttc --check`로 재현 — `result` 블록/match arm 안의 깨진 TS는 panic,
  통과 영역의 같은 오류는 정상 진단임을 확인해 경로 차이를 특정했다.
- 2026-08-24: `src/program_syntax.rs` — `ProgramSyntaxError::SourceNotTypeScript`
  추가, `parse_module`에 segment 표를 넘기고 `parse_failure`가 멈춘 바이트로 원인을
  분류하도록 변경, `source_byte_for_projection` 추가(기존 `source_span_for_projection`
  옆, 매핑 소유자 유지).
- 2026-08-24: `src/codegen/core.rs` — `lowering_plan`(+ `SourceNotTypeScript`) 분리,
  `emit_with_map`은 `&LoweringPlan`을 받고 `source_kind` 인자 제거. `src/codegen/mod.rs`
  export 갱신.
- 2026-08-24: `src/verify.rs` — `in_source` 추가(문안과 위치의 단일 소유자),
  `at_source`(출력 자가 검사)는 문안·동작 그대로 유지.
- 2026-08-24: `src/diagnostics.rs` — `SourceNotTypeScript` 코드 추가, wire form
  `source-not-typescript`, `blocks_projection()`에 포함.
- 2026-08-24: `src/lib.rs` — 세 소비자 갱신. `emit_mapped`(강등), `compile_mapped`
  (첫 에러 반환), `compile_report`(이미 찾은 진단과 함께 보고 후 emit 없음).
- 2026-08-24: 테스트 추가 — `tests/compile.rs` 5건(위치·코드·다중 진단 동시 보고·
  `--no-verify` 비우회·tt 구문 없는 파일은 기존 backstop), `tests/emit_map.rs` 1건
  (infallible 계약 + 매핑 불변식), `src/program_syntax.rs` 단위 2건(원인 분류,
  placeholder 바이트는 소스로 매핑되지 않음).
- 2026-08-24: 문서 갱신 — `docs/design/program-lowering.md` §11.1(분류 규칙 표),
  `docs/ai/tt.md`(사용자 표면: 새 진단과 `--no-verify` 비적용).
- 2026-08-24: 검증 게이트 — `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test` 전부 통과(총 756건). 정상 파일의 방출 결과가 변경 전과
  바이트 동일함을 `diff`로 확인했다.

## 이슈 및 해결

### 이슈 1: 첫 구현이 정상 클레임된 `match`를 범인으로 지목

- **증상**: `const x = match (s) { A(v) => { const q = ; return q; }, _ => 0 };`가
  ``1:11: `match` here did not parse as a tt `match` ...``로 보고됐다. 실제 오류는
  1:43의 `;`이고 match는 정상 파싱됐다.
- **원인**: `verify::at_source`의 문안 선택기 `tt_construct_at`이 오류 줄 전체에서
  `match`/`try`/`result`/`flow` 문자열을 찾는 휴리스틱이라, 클레임된 구문과 클레임에
  실패한 lookalike를 구분하지 못한다.
- **해결**: projection 경계에서는 이 추정을 쓰지 않는다(결정 5). 메시지는 멈춘
  바이트와 그 결과만 말하고, 위치는 정확히 그 바이트다. 회귀 테스트
  `invalid_typescript_in_a_match_arm_body_reports_the_byte_not_the_construct`가
  "`did not parse as a tt`를 포함하지 않는다"까지 고정한다.

### 이슈 2: `engine_cache`의 기존 테스트가 이 환경에서 실패

- **증상**: `an_error_node_keeps_its_file_and_other_files_checkable`가 진단 2개 대신
  1개를 받아 실패했다.
- **원인**: 이 테스트는 정상 파일 쪽 진단을 TypeScript 백엔드에서 받는다. 컨테이너에
  TypeScript 7이 없어서 백엔드가 없는 상태였다. 변경 전 소스(`git stash`)로도 동일하게
  실패하는 것을 확인해 이번 변경과 무관함을 확정했다.
- **해결**: typescript-go를 소스 빌드해 `TTC_TSGO_ROOT`로 지정한 뒤 전체 테스트를
  재실행했고 모두 통과했다. 코드 변경은 없다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (TTC_TSGO_ROOT로 소스 빌드한 tsgo 지정, 756건 통과)

## 결과

클레임된 tt 구문 안의 잘못된 TypeScript는 이제 panic이 아니라 멈춘 바이트를 가리키는
`.tt` 좌표 진단(`source-not-typescript`)으로 보고되고, 그 파일은 출력을 만들지 않는다.
emission은 다시 실패할 수 없는 단계가 됐고, 실패 가능한 절반은 진단을 소유한 단계가
실행한다. 컴파일러가 만든 placeholder에서의 parse 실패는 그대로 internal compiler
error로 남아 ttc 버그 신호가 약해지지 않는다.
