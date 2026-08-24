# TASK-205: VS Code 전체 도구 체인 테스트 복구

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: 이 커밋

## 목적

TASK-204에서 확정한 match arm completion ICE를 구조적으로 제거하고, 현재 엔진
계약과 어긋난 확장 테스트를 정정해 전체 도구 체인 테스트를 0 실패·0 건너뜀으로
복구한다.

## 범위

- 포함:
  - 미완성 match arm body가 completion 전에 owner construction ICE를 내지
    않도록 parser-owned recovery 또는 probe 경계를 수정.
  - macOS lexical/canonical 경로를 동일 파일 identity로 비교하는 테스트 계약.
  - pipeline probe와 sidecar projection recovery의 현재 계약을 검증하는 테스트.
  - tsgo 주입 전체 확장 테스트와 저장소 필수 게이트.
- 제외:
  - 진단 억제나 문자열 휴리스틱으로 ICE를 숨기는 처리.
  - completion·sidecar의 사용자 계약 변경.

## 의사결정

### 결정 1: parse error를 강등하지 않고 생성 경계의 provenance를 보강한다

- **상황**: 미완성 arm body `radius.`를 owner projection에 넣으면 SWC는 원본의
  점이 아니라 컴파일러가 뒤에 붙인 `)`에서 `Expected ident`를 보고했다. 기존
  분류는 오류 바이트가 placeholder에 있다는 이유로 컴파일러 불변식 위반으로
  판정해 panic했다.
- **검토한 대안**: 모든 `ProgramSyntaxError::Parse`를 입력 오류로 강등하면 실제
  생성 코드 결함도 숨긴다. completion 요청에서만 panic을 잡거나 문자열 모양을
  검사하면 다른 언어 서비스·emit-map 경로에 같은 결함이 남고 입력별 휴리스틱이
  된다. parser-owned recovery node를 추가하면 tt 파서가 일반 TypeScript의 미완성
  식까지 판정해야 한다.
- **선택과 근거**: owner projection이 원본 조각을 감싸며 쓰는 고정 닫기
  구분자를 `SourceBoundary`로 기록한다. SWC가 이 경계에서 멈춘 경우에만 바로 앞
  copied segment의 마지막 바이트를 원인으로 돌린다. 생성 placeholder 내부의
  실패는 계속 내부 오류이므로 불변식 검증을 약화하지 않는다. 동일 원리는 match
  subject·guard·expression/block arm과 template interpolation에 적용했다.

### 결정 2: 파일 위치 테스트는 파일 identity를 비교한다

- **상황**: macOS에서 `os.tmpdir()`가 만든 lexical `/var/...` 경로와 엔진이
  canonicalize한 `/private/var/...` 경로가 같은 파일인데도 네 assertion이 문자열
  차이로 실패했다.
- **검토한 대안**: 엔진이 lexical 입력 경로를 보존하면 프로젝트 identity의
  canonical 경로 계약과 충돌한다. 테스트에서 특정 접두사를 치환하면 macOS 전용
  문자열 휴리스틱이 된다.
- **선택과 근거**: definition·references·rename의 양쪽 경로를
  `fs.realpathSync`로 해석한 뒤 비교한다. 파일시스템이 보장하는 동일 파일
  identity를 직접 검증한다.

### 결정 3: 제품이 소비하는 completion·sidecar 계약만 검증한다

- **상황**: pipeline의 `member=false` 응답과 parser recovery 이전의 sidecar
  실패 기대가 현재 제품 경로와 달랐다.
- **검토한 대안**: plain completion을 빈 배열로 강제하면 TypeScript가 유효한
  recovery projection에서 제공하는 전역 응답을 인위적으로 제거한다. stray pipe
  하나 때문에 파일 전체 선언 방출을 막으면 부분 projection recovery 계약을
  되돌린다.
- **선택과 근거**: pipeline 테스트는 `member=true` probe가 실제 값의 멤버만
  반환하는지 검증한다. sidecar 테스트는 오류 노드 밖의 독립 선언이 다시
  방출되어 `written`이 되는지 검증한다.

## 작업 내역

- 2026-08-24: TASK-204의 입력으로 `ttc --emit-map`을 실행해
  `src/codegen/core.rs`의 `ProgramSyntaxError::Parse` panic과 실제 owner
  projection `...(radius.);...`를 재현했다.
- 2026-08-24: `ProjectionSegmentKind::SourceBoundary`와
  `push_source_boundary`를 추가했다. 원본 조각 직후의 생성 닫기 구분자에 parse
  cause provenance를 기록하고, 일반 source mapping에서는 이 메타데이터를
  제외했다.
- 2026-08-24: 미완성 match arm의 SWC 오류가 원본 점으로 분류되는 단위 회귀
  테스트를 추가했다. 같은 입력의 emit-map 경로가 panic하지 않음도 확인했다.
- 2026-08-24: macOS 경로 assertion 4건을 realpath identity 비교로 바꿨다.
  pipeline plain completion assertion과 sidecar의 파일 전체 실패 기대를 현재
  제품 계약으로 정정했다.
- 2026-08-24: 직접 관련된 VS Code 테스트 45건과 전체 112건을 로컬
  typescript-go checkout을 명시해 실행했다. 모두 통과했고 건너뜀은 없었다.
- 2026-08-24: 저장소 필수 Rust 게이트를 모두 실행했다.

## 이슈 및 해결

### 이슈 1: 오류 위치가 생성된 닫기 괄호에 놓였다

- **증상**: `Circle(radius) => radius.,`의 owner projection은
  `TypeScript owner construction failed: Parse { message: "Expected ident" }`로
  panic했다.
- **원인**: `radius.`는 copied segment였지만 SWC의 중단 위치는 그 식을 닫는
  컴파일러 생성 `)`였다. 기존 provenance에는 생성 구분자와 직전 입력 조각의
  문법 경계 관계가 없었다.
- **해결**: 생성된 source boundary를 별도 segment kind로 모델링하고 그
  경계에서의 parse failure만 직전 copied source의 마지막 바이트에 귀속했다.

### 이슈 2: pipeline plain completion이 전역 항목을 반환했다

- **증상**: 관련 테스트를 재실행하자 `member=false` 응답에 브라우저 전역 항목이
  있어 빈 배열 assertion이 실패했다.
- **원인**: recovery projection이 TypeScript로 parse 가능해지면서 checker가
  plain completion을 정상적으로 답했다. 에디터의 점 뒤 요청은
  `member=true`이고 이 응답을 사용하지 않는다.
- **해결**: 비제품 응답의 내용 assertion을 제거하고 probe 사용 여부와 number
  member 결과를 검증했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (828 통과)
- [x] tsgo 주입 VS Code 확장 테스트 (112 통과·0 실패·0 건너뜀)

## 결과

owner projection은 copied TypeScript 조각과 컴파일러가 붙인 문법 경계의 원인
관계를 보존한다. 미완성 match arm completion은 서버를 종료하지 않고 pattern
binding의 타입에 맞는 멤버를 반환한다. VS Code 전체 도구 체인 테스트는
112 통과·0 실패·0 건너뜀으로 복구됐다.
