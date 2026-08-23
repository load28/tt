# TASK-165: `.rlx` 에디터 기능 parity

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: —

## 목적

`.rlx` 파일이 `.rl`과 같은 수준의 VS Code 언어 기능을 제공하도록 에디터 경계를
검증하고 보강한다. TSX 하이라이팅과 파일 아이콘뿐 아니라 진단, 완성, hover,
정의·참조·rename, signature help, semantic token, sidecar를 회귀 테스트로 고정한다.

## 범위

- 포함: `.rlx` 전용 파일 아이콘, TSX language-service 문서 종류, VS Code/LSP
  `.rlx` parity 테스트, sidecar와 manifest 계약, 관련 문서.
- 제외: 새로운 rl 문법, JSX runtime 제공, 다른 에디터용 별도 확장.

## 의사결정

### 결정 1: 별도 LSP를 만들지 않고 동일 엔진에 소스 종류를 전달한다

- **상황**: `.rlx` 기능을 `.rl` 구현에 합칠지 별도 서버로 만들지 선택해야 한다.
- **검토한 대안**: 별도 서버는 TSX 설정을 독립시킬 수 있지만 진단·완성·탐색
  기능이 복제된다. 동일 서버는 문서 확장자로 TS/TSX 종류만 구분하면 모든 의미
  기능과 source mapping을 공유한다.
- **선택과 근거**: 동일 서버와 엔진을 유지한다. projected URI가 `.tsx`이면 native
  TypeScript service에 `typescriptreact` 문서로 열고 기능별 parity 테스트로
  동일한 요청 경로를 증명한다.

### 결정 2: `.rlx`는 식별 가능한 전용 아이콘을 사용한다

- **상황**: manifest에는 `.rlx` 아이콘 항목이 있지만 `.rl`의 `RL` 그림을 그대로
  재사용해 탐색기에서 두 소스 종류를 구분할 수 없다.
- **검토한 대안**: 같은 아이콘 재사용은 파일 유형 등록만 충족한다. `RLX` 전용
  자산은 파일 목록에서 TSX 소스 종류를 즉시 구분할 수 있다.
- **선택과 근거**: light/dark 전용 SVG를 추가하고 manifest 계약 테스트에서
  `.rlx`가 그 자산을 가리키는지 고정한다.

## 작업 내역

- 2026-08-23: TASK-163 구현과 VS Code manifest, client selector, TextMate grammar,
  native engine, sidecar와 테스트 범위를 조사했다. 등록 경로는 존재하지만 LSP
  기능별 `.rlx` 회귀 검증과 전용 아이콘이 없음을 확인했다.
- 2026-08-23: native TypeScript service가 projected `.tsx` URI를
  `typescriptreact`로 열도록 문서 종류 결정을 추가하고 단위 테스트로 고정했다.
- 2026-08-23: light/dark `RLX` 아이콘을 추가하고 VS Code manifest가 `.rlx`에
  전용 아이콘과 TextMate grammar를 연결하는지 테스트했다.
- 2026-08-23: 실제 `.rlx`/JSX 문서로 타입 진단, hover, 멤버 완성, 정의, 참조,
  rename, signature help, rl 심볼·완성, semantic token을 검증하는 engine 테스트와
  LSP 프로토콜 통합 테스트를 추가했다.
- 2026-08-23: `.rlx` 저장 시 TSX declaration과 source map을 갱신하고 원본
  `.rlx`로 매핑하는 sidecar 테스트를 추가했다.
- 2026-08-23: 확장 README와 설정 설명, changelog에 `.rlx` 지원 범위를 반영했다.
- 2026-08-23: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`와 `.rlx` 관련 VS Code 테스트 5개를 실행해 모두 통과했다.

## 이슈 및 해결

- VS Code manifest 수정 중 동일한 아이콘 블록이 먼저 일치해 `.rl` 항목이 잠시
  `.rlx` 아이콘을 가리켰다. 각 language id를 확인해 `.rl`과 `.rlx` 항목을
  분리하고 manifest 계약 테스트로 고정했다.
- macOS 임시 디렉터리의 `/var` 경로가 definition 결과에서 `/private/var`로
  정규화되어 테스트가 실패했다. 양쪽 경로를 `realpath`로 비교해 동일 파일이라는
  의미를 검증하도록 수정했다.
- LSP 타입 진단의 원본 범위는 선언 이름이 아니라 잘못 대입된 우변 `label`이었다.
  실제 source mapping 계약에 맞춰 기대 범위를 조정했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `.rlx` VS Code engine/grammar/LSP/sidecar 테스트

## 결과

`.rlx`가 VS Code에서 TSX 기반 구문 하이라이팅과 전용 파일 아이콘을 사용한다.
동일한 언어 서버에서 rl 의미 기능과 TypeScript React 의미 기능을 함께 제공하며,
진단·완성·탐색·리팩터링·semantic token·sidecar 동작을 회귀 테스트로 고정했다.
