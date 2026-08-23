# TASK-180: malformed projection의 codegen 진입 차단

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: 7d5e1a3

## 목적

malformed tt 구문과 정상 tt 구문이 한 파일에 있을 때 복구 전에 codegen이 실행돼
compiler server가 종료되는 문제를 고친다. CI가 정상 종료되어 대기 중인 개발
배포가 시작되게 한다.

## 범위

- 포함: malformed enum·match의 일반 emission 차단, 에디터 projection 복구,
  compiler panic과 LSP 진단 회귀 테스트, CI 재검증
- 제외: 실제 확장 language server 프로토콜 변경, 진단 내용 변경

## 의사결정

### 결정 1: malformed parser node는 일반 emission을 차단

- **상황**: malformed `match` 자체는 진단됐지만 같은 파일의 정상 tt `enum`이
  Core IR host lowering을 요구해, 유효하지 않은 원문 전체가 ProgramSyntax로
  전달되면서 compiler server가 panic했다.
- **검토한 대안**: codegen panic을 잡아 진단으로 바꾸면 계층 불변식을 숨긴다.
  malformed 원문을 codegen에서 치환하면 parser recovery 책임이 backend로 샌다.
  일반 emission을 차단하고 editor projection만 parser recovery node를 치환하면
  기존 `compile_projection_report` 계약을 그대로 사용한다.
- **선택과 근거**: `MalformedEnum`과 `MalformedMatch`를
  `DiagnosticCode::blocks_projection()`에 포함한다. 일반 `compile_report`는 emit을
  만들지 않고, editor projection은 byte-length preserving recovery 후 다시
  컴파일한다.

### 결정 2: 혼합 입력을 두 계층 회귀 테스트로 고정

- **상황**: malformed match만 있는 단위 테스트는 Core host lowering이 필요 없어
  panic을 재현하지 못했다.
- **검토한 대안**: LSP timeout만 테스트하면 실패 원인이 간접적이다. compiler
  report만 검사하면 editor recovery 결과가 빠진다.
- **선택과 근거**: 정상 tt enum과 malformed match를 함께 둔 입력으로 일반
  report의 emission 차단과 editor projection의 복구 emission을 각각 검사한다.

## 작업 내역

- 2026-08-23: GitHub CI 두 job과 로컬 전체 확장 테스트에서 `server.test.js`의
  진단 대기가 60초 timeout되는 현상을 재현했다.
- 2026-08-23: 동일 입력을 CLI로 실행해 `TypeScript owner construction failed`
  panic을 재현하고, malformed match와 정상 enum의 조합이 host lowering에 진입한
  원인임을 확인했다.
- 2026-08-23: malformed enum·match를 projection 차단 진단으로 분류하고 일반
  compile report와 editor recovery projection 회귀 테스트를 추가했다.
- 2026-08-23: 실패했던 LSP 테스트 두 건이 1.5초 안에 종료하며 모두 통과함을
  확인했다.
- 2026-08-23: `cargo fmt --check`, clippy, 전체 Rust 테스트를 통과했다.

## 이슈 및 해결

### 이슈 1: malformed 원문이 host lowering에서 compiler를 panic시킴

- **증상**: `a construct that did not parse is reported where it is written`가
  60초 후 timeout됐다. CLI는 `TypeScript owner construction failed`로 panic했다.
- **원인**: `MalformedMatch`가 projection을 차단하지 않아 정상 enum의 host
  lowering이 malformed TypeScript 원문까지 파싱했다.
- **해결**: malformed parser diagnostics가 일반 emission을 차단하고 editor
  projection recovery가 먼저 적용되도록 진단 계약을 바로잡았다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

malformed enum·match가 있는 파일은 일반 codegen에 진입하지 않는다. 에디터는
parser recovery node를 먼저 치환한 뒤 정상 tt 구문과 독립 TypeScript 영역을
계속 투영하며, compiler server는 panic 없이 원래 malformed 진단을 반환한다.
