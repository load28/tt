# TASK-204: VS Code 전체 도구 체인 테스트 실패 조사

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: —

## 목적

로컬 tsgo 도구 체인을 명시적으로 주입한 VS Code 확장 테스트 112건 중 실패한
7건을 재현하고, 제품 결함·테스트 결함·실행 환경 문제를 구분해 근본 원인을
확정한다.

## 범위

- 포함:
  - 실패 7건의 단독·묶음 재현과 병렬 실행 영향 확인.
  - 경로 정규화, completion 응답, sidecar 실패 판정의 실제/기대값 대조.
  - 원인별 수정 범위와 회귀 테스트 요구사항 확정.
- 제외:
  - 원인이 확정되기 전의 구현 변경.
  - 조사 결과와 무관한 에디터 기능 변경.

## 의사결정

### 결정 1: 실패 개수가 아니라 계약과 직접 재현 결과로 분류한다

- **상황**: 전체 실행에서는 7건이 함께 실패하지만 경로 비교, completion,
  sidecar는 서로 다른 계층이다. 한 회귀로 묶으면 테스트 기대값 변경과 제품
  결함을 구분할 수 없다.
- **검토한 대안**: 전체 로그만 기준으로 하나의 회귀로 처리 / 각 테스트 파일을
  단독 실행하고 공개 엔진 프로토콜까지 내려가 실제 응답을 확인.
- **선택과 근거**: 후자. 네 테스트 파일을 각각 실행해 같은 2·1·3·1 실패를
  재현했으므로 병렬 실행은 배제했다. completion은 `ttc --server`에 요청을
  직접 보내 프로세스 종료 원인까지 확인했다.

### 결정 2: 제품 결함 1건과 오래된 테스트 계약 6건을 분리한다

- **상황**: 실패가 곧 제품 결함인 것은 아니며, TASK-142의 오류 노드 복구와
  macOS 경로 정규화처럼 현재 계약이 테스트 작성 당시와 달라진 부분이 있다.
- **검토한 대안**: 기대값을 모두 현재 출력에 맞춤 / 현재 아키텍처 계약을
  기준으로 실제 사용자 기능이 깨진 경우만 제품 결함으로 판정.
- **선택과 근거**: 후자. match arm completion은 서버가 panic하므로 제품
  결함이다. 나머지는 실제 위치가 같은 경로의 문자열 차이 4건, 제품 경로에서
  쓰지 않는 plain completion의 빈 배열 가정 1건, projection 복구 이전의
  sidecar 실패 가정 1건이다.

## 작업 내역

- 2026-08-24: `.tt-dev/toolchain.json`의 checkout을 `TTC_TSGO_ROOT`·
  `TTC_TSGO_BIN`·`TTC_TSGO_API`로 주입해 전체 확장 테스트를 실행했다.
  112건 중 105건 통과, 7건 실패를 확인했다.
- 2026-08-24: `completion.test.js`, `emitmap.test.js`, `engine.test.js`,
  `sidecar.test.js`를 각각 단독 실행했다. 각각 2·1·3·1건이 그대로 실패했다.
- 2026-08-24: 로컬 typescript-go가 CI 고정 커밋
  `c6b013f5706d58582f566df778cc0df2683b58f5`와 일치하고 실행 파일/API가 같은
  checkout에서 빌드됐음을 확인했다.
- 2026-08-24: `cargo build`로 `target/debug/ttc`를 현재 HEAD에서 다시 만든 뒤
  전체 테스트를 재실행했다. 결과는 105 통과·7 실패로 같았다.
- 2026-08-24: match arm completion을 `ttc --server` JSON 프로토콜로 직접
  요청해 `src/codegen/core.rs:67`의 owner construction panic을 재현했다.
- 2026-08-24: TASK-142의 projection 전용 오류 노드 복구와 sidecar의 종료 코드
  계약을 대조해 stray pipe가 이제 exit 1 + declaration 재방출 대상임을 확인했다.

## 이슈 및 해결

### 이슈 1: 미완성 match arm completion이 서버를 종료한다

- **증상**: `Circle(radius) => radius.,`의 점 뒤 completion이 빈 결과를
  반환한다. 직접 요청에서는 `src/codegen/core.rs:67`에서
  `TypeScript owner construction failed: Expected ident` panic이 발생한다.
- **원인**: completion probe를 만들기 전에 서비스 문서가 미완성 arm body를
  whole-owner `ProgramSyntax`로 구성한다. `radius.`가 아직 TypeScript 식이 아닌
  상태를 owner construction의 실패 가능한 입력으로 전달하고, 그 경계가
  `Result`가 아니라 ICE로 닫혀 있어 서버 전체가 종료된다.
- **해결**: 조사 범위에서는 구현하지 않는다. parser-owned recovery 또는
  completion의 probe-first 경계에서 미완성 식을 구조적으로 수용하고, 서버가
  panic하지 않는 회귀 테스트를 TASK-205에 확정했다.

### 이슈 2: macOS 임시 경로의 lexical/canonical 표기가 다르다

- **증상**: definition·references·rename 테스트 4건에서 실제 경로는
  `/private/var/...`, 기대 경로는 `/var/...`다.
- **원인**: macOS의 `/var`는 `/private/var`를 가리킨다. 엔진은 프로젝트
  identity와 응답 경로를 canonicalize하지만 테스트 4건은 `os.tmpdir()`로 만든
  lexical 경로를 문자열로 직접 비교한다.
- **해결**: 제품 위치는 동일하다. 테스트가 이미 같은 파일의 다른 사례에서
  사용하는 `fs.realpathSync` 계약으로 비교하도록 TASK-205 범위에 넣었다.

### 이슈 3: pipeline 테스트가 제품 경로 밖의 plain 응답을 제한한다

- **증상**: `x |> .`에서 `member=false`로 직접 요청한 plain completion에 전역
  항목이 있어 빈 배열 기대가 실패한다. `member=true` 제품 경로는 probe를
  만들고 `toFixed`·`toString`·`toPrecision`을 모두 반환한다.
- **원인**: 현재 recovery projection은 미완성 문장도 parseable TypeScript로
  제공하므로 TypeScript가 전역 completion을 답할 수 있다. 에디터는 점 뒤에서
  항상 `member=true`로 요청하고 엔진이 전역 응답을 버리므로 plain 배열의
  내용은 사용자 계약이 아니다.
- **해결**: probe가 실제로 사용되고 member 목록만 반환된다는 계약만 검증하도록
  TASK-205에서 오래된 assertion을 제거한다.

### 이슈 4: sidecar 테스트가 projection 복구 이전 계약을 기대한다

- **증상**: stray pipe가 추가된 파일의 refresh 결과가 기대한 `failed`가 아니라
  `written`이다.
- **원인**: TASK-142 이후 engine projection은 stray pipe 오류 노드만 안전한
  placeholder로 바꾸고 파일의 나머지 선언을 계속 검사·방출한다. `--types`는
  진단과 선언을 함께 내고 exit 1이며, sidecar는 exit 1을 `written`으로
  처리한다. 테스트 설명은 파일 전체가 projection에서 빠지던 이전 계약이다.
- **해결**: 정상 컴파일의 emission-withholding 계약은 그대로 두고, editor
  projection의 부분 복구 계약에 맞게 테스트를 TASK-205에서 갱신한다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (827 통과)
- [x] tsgo 주입 VS Code 확장 테스트 (105 통과·7 실패·0 건너뜀, 조사 기준선)

## 결과

실패 7건을 세 원인군으로 분리했다. 실제 제품 결함은 미완성 match arm
completion의 서버 ICE 1건이다. 경로 비교 4건, pipeline plain 응답 1건,
sidecar recovery 기대 1건은 현재 계약을 반영하지 못한 테스트다. 수정과 전체
게이트 복구는 [TASK-205](./TASK-205-vscode-full-toolchain-test-fixes.md)에서 한다.
