# TASK-245: `variant` 기반 tt 태그드 유니언 선언

- **상태**: 완료
- **시작일**: 2026-08-26
- **완료일**: 2026-08-26
- **커밋**: —

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

## 이슈 및 해결

- 첫 `./scripts/ci`에서 변환되지 않은 `enum B` 테스트 입력 때문에 resolve 테스트가
  실패했다. 해당 선언을 `variant B`로 고쳐 두 선언의 식별자 계약을 복원했다.
- 두 번째 `./scripts/ci`에서 키워드 길이 변화가 반영되지 않은 sidecar 원본 열
  기대값 때문에 실패했다. 선언명 시작 열을 12에서 15로 갱신했다.
- 최초 완료 검토에서 공개 API와 도구 JSON에 `enum` 명칭이 남은 것을 확인했다.
  호환 별칭 없이 전체 소비자를 `variant` 계약으로 함께 바꿨다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## 결과

tt 태그드 유니언은 문법, 컴파일러 모델, 공개 Rust API, ttc·engine 프로토콜,
VS Code와 문서에서 `variant`로만 표현한다. TypeScript와 LSP 표준이 소유하는
`enum`·`enumMember`만 원래 의미로 유지한다. 괄호 없는 유닛 케이스와 기존
페이로드·제네릭·match 의미는 동일하게 동작한다.
