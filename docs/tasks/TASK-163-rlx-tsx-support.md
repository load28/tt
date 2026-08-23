# TASK-163: `.rlx` — TSX 위의 rl 문법과 React 도구 체인

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: —

## 목적

모든 유효한 TSX를 그대로 받아들이면서 JSX expression container 안에서도 rl 구문을
사용할 수 있는 `.rlx` 소스 종류를 추가한다. 컴파일러 코어부터 프로젝트 그래프,
타입 검사, 에디터와 번들러까지 하나의 소스 종류 계약을 공유하게 한다.

## 범위

- 포함: 이름 있는 TS/TSX 소스 종류, 구조적인 JSX lexical boundary, `.rlx` → `.tsx`
  출력, `.rlx` import와 프로젝트 그래프, 타입 검사·VS Code·unplugin 연동, 세 계층
  테스트와 사용자 문서.
- 제외: React 런타임 내장, JSX 변환 자체, 특정 JSX 라이브러리에 종속된 코드 생성,
  기존 `.rl`에서 JSX 허용.

## 의사결정

### 결정 1: `.rlx`를 React 전용 문법이 아니라 TSX 소스 종류로 정의한다

- **상황**: React 지원을 컴파일러 기능으로 넣을지, JSX 보존 계약으로 넣을지 선택해야
  한다.
- **검토한 대안**: React import와 JSX runtime을 rlc가 관리하면 초기 설정은 줄지만
  React 버전과 runtime 모드에 결합한다. TSX를 그대로 보존하면 React·Preact 등 JSX
  소비자가 기존 TypeScript 설정을 유지하지만 프로젝트가 `jsx` 옵션을 정해야 한다.
- **선택과 근거**: `.rlx = TSX + rl`로 정한다. rlc는 소유한 구문만 변환한다는 기존
  통과 계약과 에러 계층을 그대로 유지하고 JSX 의미와 emit은 TypeScript에 둔다.

### 결정 2: 확장자 분기를 소스 종류 값으로 정규화한다

- **상황**: parser, SWC 검증, 프로젝트 모듈명과 출력 확장자가 모두 TSX 여부를 알아야
  한다.
- **검토한 대안**: 각 계층에서 파일명 suffix를 검사하면 빠르지만 동일한 정책이
  흩어지고 문자열 기반 예외가 된다. 공개 `SourceKind`를 전달하면 API 호출자도 파일명
  없이 문법을 정확히 지정할 수 있다.
- **선택과 근거**: 공개 `SourceKind::{TypeScript, Tsx}`를 단일 원천으로 두고 CLI
  경계에서 확장자를 이 값으로 변환한다. 이후 단계는 확장자를 다시 해석하지 않는다.

### 결정 3: JSX는 raw와 expression container를 구조적으로 분리한다

- **상황**: 현재 lexer는 JSX 텍스트도 식별자 토큰으로 만들기 때문에 raw text의
  `match`를 rl 구문으로 오인할 수 있다.
- **검토한 대안**: 후보 문자열 주변의 `<`/`>`를 검사하는 방식은 중첩 JSX, fragment,
  spread와 TSX의 `<` 모호성을 일반화하지 못한다. JSX lexical context를 이름 있는
  구조로 모델링하면 raw는 불투명하게 보존하고 `{...}`만 기존 parser로 재귀 처리할 수
  있다.
- **선택과 근거**: TSX 모드 lexer에 JSX raw/expression 경계를 추가하고 AST·codegen은
  그 경계를 보존한다. 최종 출력은 TSX 모드 SWC 파서로 다시 검증한다.

### 결정 4: JSX 평가와 concise arrow를 기존 owner/protocol 모델로 표현한다

- **상황**: JSX 속성·자식 안의 match를 statement로 승격하면 앞선 속성의 부작용
  순서와 expression-bodied arrow의 렉시컬 범위를 보존해야 한다.
- **검토한 대안**: JSX에서만 IIFE를 넣으면 동작은 하지만 기존 whole-owner lowering
  계약을 우회한다. JSX를 Evaluation IR의 eager position으로, concise arrow body를
  host owner로 만들면 다른 식 위치와 같은 합성 규칙을 적용할 수 있다.
- **선택과 근거**: `Jsx` protocol frame과 `ArrowExpression` owner를 추가했다. codegen은
  이 계획에 따라 선행 식을 한 번 평가하고 arrow expression body를 block+return으로
  낮춘다. 통합 테스트에서 타입 검사와 좌우 평가 순서를 확인했다.

## 작업 내역

- 2026-08-23: 현재 compiler API, lexer, SWC syntax boundary, 프로젝트 수집·가상 모듈,
  CLI 출력, unplugin과 VS Code 확장자 경계를 조사했다.
- 2026-08-23: TASK-163을 등록하고 `.rlx = TSX + rl`, `SourceKind`, 구조적 JSX 경계를
  구현 원칙으로 확정했다.
- 2026-08-23: `SourceKind`를 parser·HIR·Core IR·SWC 검증·engine projection까지
  전달했다. `.rlx` import는 종류를 보존해 `.jsx` 또는 `.tsx`로 방출하도록 했다.
- 2026-08-23: JSX raw/expression 경계를 lexer에 구현하고, ProgramSyntax와 Evaluation
  IR에 JSX 평가 위치와 concise-arrow owner를 추가했다. 정규식 안 구분자는 공용
  balanced scanner가 건너뛰도록 렉서와 같은 regex 판별 계약을 공유했다.
- 2026-08-23: CLI 프로젝트 수집·출력, native TypeScript 프로젝트, VS Code 언어
  등록·LSP·TextMate 문법, unplugin 가상 모듈을 `.rlx`/`.tsx` 경로에 연결했다.
- 2026-08-23: compile·passthrough·CLI·integration·native와 VS Code grammar 회귀
  테스트를 추가했다. README, 내장 AI 가이드, 아키텍처 문서, 에디터·플러그인 문서,
  웹사이트 영문·한글 콘텐츠를 갱신했다.
- 2026-08-23: Rust 필수 게이트 전체, VS Code grammar 생성·컴파일·관련 테스트,
  unplugin 구문 검사, 웹사이트 타입 검사와 29개 페이지 prerender를 통과했다.
- 2026-08-23: 형제 React 예제 `rlx-tour`를 만들고 실제 소비 프로젝트에서 발견한
  handwritten `.tsx` source-kind 경계를 보완했다. 로컬 `rl-lang`·`unplugin-rl`을
  현재 저장소로 재설치한 뒤 check·typecheck·Vitest·Vite build를 통과했다.

## 이슈 및 해결

- **증상**: JSX를 반환하는 concise arrow 안의 match가 모듈 statement 앞으로
  이동해 arrow 매개변수를 찾지 못했다. **원인**: 기존 owner 모델이 expression-bodied
  arrow 자체를 소유권 경계로 표현하지 않았다. **해결**: `ArrowExpression` owner와
  block+return lowering을 추가하고 일반 arrow 회귀 테스트로 고정했다.
- **증상**: JSX expression container의 정규식 안 `}`가 container 끝으로 해석될 수
  있었다. **원인**: balanced scanner와 lexer가 regex 판별 계약을 공유하지 않았다.
  **해결**: regex 시작 판정을 scanner의 공용 규칙으로 옮기고 balanced scan에서도
  regex 리터럴을 원자적으로 건너뛰게 했다.
- **증상**: 홈페이지 빌드의 prerender 단계가 샌드박스에서 `listen EPERM ::1`로
  실패했다. **원인**: preview 서버의 로컬 포트 바인딩 제한이다. **해결**: 승인된
  샌드박스 외부에서 같은 빌드를 다시 실행해 29개 페이지를 확인했다.
- **증상**: VS Code 전체 테스트는 현재 저장소의 외부 toolchain 의존 테스트가
  장시간 대기해 중단했다. **원인**: 로컬 TypeScript 7/rlc 세션 의존성이다.
  **해결**: 이번 변경이 소유한 grammar 생성 검사, TypeScript 컴파일, TSX parity와
  JSX expression scope 테스트를 독립 실행해 모두 통과시켰다.
- **증상**: `rlc --check src`가 함께 수집한 handwritten `.tsx`를 TS 문법으로
  검증했다. **원인**: `SourceKind::from_path`가 컴파일러 소유 확장자만 분류하고
  TypeScript-family 입력 확장자는 분류하지 않았다. **해결**: syntax kind 분류와
  rl-owned output 분류를 `from_path`/`from_rl_path`로 분리하고 CLI 회귀 테스트로
  `.tsx` 확장자와 JSX 통과를 고정했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

`.rlx = TSX + rl` 소스 종류를 컴파일러 코어부터 CLI, native 프로젝트, VS Code,
unplugin까지 연결했다. 유효한 TSX는 바이트 그대로 통과하고 JSX expression
container의 rl 구문은 소스 평가 순서와 함수 범위를 보존해 `.tsx`로 낮아진다.
React·Preact 같은 JSX runtime 선택과 변환은 기존 TypeScript 설정의 책임으로 남는다.
