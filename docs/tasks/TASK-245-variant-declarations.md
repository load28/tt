# TASK-245: `variant` 기반 tt 태그드 유니언 선언

- **상태**: 완료
- **시작일**: 2026-08-26
- **완료일**: 2026-08-27
- **커밋**: `912c729`, `166c9d8`, `7200308`, `69b0408`

## 목적

GitHub Discussion #63의 선택지 B에 따라 TypeScript `enum`과 tt 태그드 유니언의
소유권을 첫 토큰에서 분리한다. tt 태그드 유니언은 `variant`로만 선언하고 모든
`enum` 선언은 TypeScript에 맡긴다.

## 범위

- 포함: `variant` parser·AST/HIR·분석·codegen 지원, 유닛 케이스의 괄호 없는 선언,
  기존 tt `enum` 문법 제거, TypeScript enum 통과 계약, VS Code 문법 강조·자동완성,
  영문·한글 사용자 문서와 AI 가이드 갱신
- 제외: 호환 전환 기간과 deprecation 진단, 자동 마이그레이션, 런타임 표현과 `match`
  의미 변경, TypeScript `enum` 동작 변경

## 의사결정

### 결정 1: `variant`를 tt 태그드 유니언의 전용 선언 키워드로 사용

- **상황**: 현재 tt `enum`은 선언 전체의 제네릭 또는 페이로드 괄호를 확인해야
  TypeScript enum과 구분되며 유닛 전용 선언은 빈 괄호가 필요하다.
- **검토한 대안**: 현행 문맥 판별 유지 / `variant` 전용 키워드 / `enum tt` 표식.
- **선택과 근거**: Discussion #63의 선택지 B인 `variant`를 선택한다. 선언 첫 토큰에서
  tt 소유권이 확정되고 TypeScript enum을 그대로 통과시키면서 모든 유닛 케이스를
  괄호 없이 표현할 수 있다.

### 결정 2: 기존 tt `enum` 호환과 마이그레이션을 제공하지 않음

- **상황**: Discussion 댓글은 전환 기간을 제안했지만 프로젝트가 아직 1.0 이전이므로
  사용자가 마이그레이션 호환을 범위에서 제외했다.
- **검토한 대안**: 즉시 제거 / 조용한 무기한 호환 / 한 릴리스 deprecation.
- **선택과 근거**: 기존 tt `enum` 인식을 즉시 제거한다. `variant`만 ttc가 소유하고
  `enum`은 TypeScript에 전부 넘기므로 선언 경계가 단일 규칙으로 정리된다.

### 결정 3: flow의 TypeScript 키워드와 tt 문맥 키워드를 분리

- **상황**: PR 코드 리뷰에서 flow 구조 분류표의 TypeScript `enum`이 `variant`로
  교체되어 통과 계약을 깨뜨릴 수 있다는 major finding에 세 리뷰어가 합의했다.
- **검토한 대안**: 현재 교체 유지 / 두 단어를 전역 키워드로 등록 / `enum`을 복원하고
  완전히 인식된 tt `variant` 선언만 별도로 처리.
- **선택과 근거**: `enum`은 TypeScript 구조 키워드로 복원한다. `variant`는 일반
  TypeScript 식별자이므로 완전히 인식된 tt 선언 문맥에서만 flow 구조로 취급한다.

### 결정 4: 편집기 표시와 LSP wire 명칭을 분리

- **상황**: 재리뷰에서 여러 줄 TypeScript 식별자를 variant 선언으로 강조하는 문법과
  테스트의 LSP 토큰 범례를 `variant`로 바꾼 문제가 합의된 minor finding이 되었다.
- **검토한 대안**: 문법의 넓은 공백 허용 유지 / variant 선언명을 같은 줄에서만
  인식하고 선언 본문 또는 제네릭 시작을 확인 / LSP에 비표준 `variant` 토큰 추가.
- **선택과 근거**: TextMate 문법은 같은 줄의 선언명과 뒤의 `<` 또는 `{`를 확인한다.
  내부 개념은 variant로 유지하되 LSP wire token은 표준 `enum`을 유지한다.

## 작업 내역

- 2026-08-26: `./scripts/doctor`로 개발 환경을 확인했다.
- 2026-08-26: `main`을 `c88389d`까지 fast-forward하고 작업 브랜치를 만들었다.
- 2026-08-26: GitHub Discussion #63의 본문과 댓글을 확인하고 구현 범위를 정했다.
- 2026-08-26: parser와 진단을 `variant` 전용 선언으로 바꾸고 TypeScript `enum`
  통과 및 괄호 없는 유닛 케이스 계약을 회귀 테스트로 고정했다.
- 2026-08-26: VS Code 문법·스니펫·언어 서비스와 영문·한글 문서 및 설계 문서를
  `variant` 표면에 맞췄다.
- 2026-08-26: 스냅샷 diff를 검토하고 전체 로컬 게이트를 통과했다.
- 2026-08-26: 공개 API와 도구 프로토콜에 남은 `enum` 명칭을 확인하여 태스크를
  다시 열었다.
- 2026-08-26: Rust 공개 API를 `ExternVariant`·`VariantSymbol`·
  `exported_variants`·`variant_symbols`와 `extern_variants`로 전환했다.
- 2026-08-26: ttc `--symbols`와 engine JSON을 `variants`·`variantName`으로
  전환하고 VS Code 소비 모델과 회귀 테스트를 함께 갱신했다.
- 2026-08-26: AST·HIR·resolve 내부 모델과 parser·설계·fixture 경로의 tt 전용
  명칭도 `variant`로 통일하고 전체 로컬 게이트를 다시 통과했다.
- 2026-08-27: PR #66을 세 관점과 진행자 App이 5라운드로 리뷰했으며, TypeScript
  `enum` 구조 분류 회귀와 VS Code 문서 잔여 용어를 반영하기로 승인받았다.
- 2026-08-27: flow의 TypeScript `enum` 분류를 복원하고 tt `variant` 선언은 완성된
  문맥에서만 인식하도록 분리했다. VS Code 문서 용어와 회귀 테스트도 갱신했다.
- 2026-08-27: `./scripts/ci` 전체 게이트를 다시 통과했다.
- 2026-08-27: 새 head 재리뷰의 합의된 편집기 minor 두 건을 반영했다. TextMate의
  ASI 회귀 테스트와 LSP `enum` wire token 검증을 추가했다.
- 2026-08-27: 원격 performance 체크가 벤치마크 입력의 폐기된 tt `enum` 때문에
  실패한 원인을 확인하고 해당 선언을 `variant`로 바꿨다.
- 2026-08-27: 편집기 minor와 벤치마크 수정 후 `./scripts/ci` 전체 게이트를 다시
  통과했다.

## 이슈 및 해결

- 첫 `./scripts/ci`에서 변환되지 않은 `enum B` 테스트 입력 때문에 resolve 테스트가
  실패했다. 해당 선언을 `variant B`로 고쳐 두 선언의 식별자 계약을 복원했다.
- 두 번째 `./scripts/ci`에서 키워드 길이 변화가 반영되지 않은 sidecar 원본 열
  기대값 때문에 실패했다. 선언명 시작 열을 12에서 15로 갱신했다.
- 최초 완료 검토에서 공개 API와 도구 JSON에 `enum` 명칭이 남은 것을 확인했다.
  호환 별칭 없이 전체 소비자를 `variant` 계약으로 함께 바꿨다.
- PR 리뷰에서 flow 키워드 표의 기계적 치환이 TypeScript `enum` 문장 경계를 잃게 한
  문제를 확인했다. `enum`은 TypeScript 구조 키워드로 복원하고 `variant`는 선언
  형태를 별도로 확인하게 했다.
- PR 리뷰에서 VS Code README에 tt 선언을 `enum`으로 부르는 표현이 남은 것을
  확인했다. TypeScript 표준 용어를 제외한 해당 표현을 `variant`로 통일했다.
- 재리뷰에서 TextMate의 `\\s+`가 줄바꿈 ASI 문장을 variant 선언으로 오인하는 문제를
  확인했다. 선언명 앞 공백을 한 줄로 제한하고 뒤에 `<` 또는 `{`가 있는지 확인했다.
- 재리뷰에서 시맨틱 토큰 테스트 범례가 실제 LSP `enum` wire token과 달라진 문제를
  확인했다. 범례를 복원하고 variant 선언명의 실제 토큰 종류를 검증했다.
- 원격 performance 체크에서 벤치마크가 폐기된 tt `enum` 문법을 사용해 컴파일되지
  않았다. 벤치마크 선언을 `variant`로 바꾸고 직접 실행해 측정을 확인했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`
- [x] `TT_BENCH_ITERS=1 cargo bench --bench compile -- --json`

## 결과

tt 태그드 유니언은 문법, 컴파일러 모델, 공개 Rust API, ttc·engine 프로토콜,
VS Code와 문서에서 `variant`로만 표현한다. TypeScript와 LSP 표준이 소유하는
`enum`·`enumMember`만 원래 의미로 유지한다. 괄호 없는 유닛 케이스와 기존
페이로드·제네릭·match 의미는 동일하게 동작한다.

변경 파일은 다음 책임 영역으로 나뉜다.

- 컴파일러: `src/parser/variants.rs`, `src/ast.rs`, `src/hir/`, `src/resolve/`,
  `src/analysis/`, `src/engine/`, `src/codegen/`, `src/flow/mod.rs`와 공개 API·CLI
- 검증: `tests/`, `tests/fixtures/`, `benches/compile.rs`
- 편집기: `editors/vscode/server/`, `editors/vscode/syntaxes/`, VS Code README
- 문서: 영문·한글 README, `docs/ai/tt.md`, 관련 `docs/design/` 문서
