# TASK-214: 패닉 안전망 — 컴파일러 버그를 버그로 보고하기

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: `ab4c2ef`

## 목적

`src/`에는 `unwrap()` 78개, `expect(` 100개, `panic!/unreachable!` 77개가 있고
`InternalCompilerError::raise()`도 패닉으로 실패를 알린다. 그런데 프로세스 경계에
`catch_unwind`도 panic hook도 없다. 그 결과:

1. CLI에서 컴파일러 버그를 밟으면 사용자는 raw Rust 백트레이스를 본다 — 자기
   코드가 잘못됐는지 컴파일러가 깨졌는지 알 수 없고, 무엇을 제보해야 하는지도
   모른다.
2. `--server` 모드에서는 한 요청의 패닉이 **세션 전체를 죽인다**. 프로토콜 문서가
   "A failed request never ends the session"이라고 약속하는데, 패닉은 그 약속
   밖에 있다. 에디터에서는 언어 서버가 조용히 사라지는 것으로 보인다.

`ice.rs`는 이미 "이건 컴파일러가 자기 계약을 깼다는 보고"라는 개념을 갖고 있다.
이 태스크는 그 개념을 lowering 검증기에서 **프로세스 경계**까지 넓힌다.

## 범위

- 포함:
  - 패닉을 구조화된 ICE 보고로 렌더하는 hook (무엇을 하던 중이었는지, 어디서,
    어느 버전, 어떻게 제보하는지)
  - CLI 진입점의 `catch_unwind` — 의도된 종료 코드
  - 서버의 요청 단위 `catch_unwind` — 세션이 살아남고 JSON 오류를 돌려준다
  - "무엇을 하던 중" 문맥(작업 중인 파일)을 남기는 스레드 로컬
- 제외:
  - `unwrap`/`expect` 자체를 줄이는 감사 (별도 태스크)
  - 패닉 시 사용자 소스를 자동 복사하는 것 (아래 결정 3)

## 의사결정

### 결정 1: 새 모듈이 아니라 `ice.rs`를 넓힌다

- **상황**: 프로세스 경계의 패닉 처리를 어디에 둘지. `src/panic.rs`를 새로
  만들 수도 있었다.
- **검토한 대안**:
  - A. 새 모듈. lowering 검증기와 무관해 보이는 코드를 섞지 않는다.
  - B. `ice.rs` 확장. 그 모듈의 주제는 "이건 컴파일러가 자기 계약을 깼다는
    보고"이고, `InternalCompilerError::raise()`는 **이미 패닉으로** 실패를
    알린다.
- **선택과 근거**: B. 검증기의 보고와 잘못된 `unwrap`은 사용자 입장에서 같은
  사건이고, 같은 보고를 받아야 한다. 실제로 `raise()`가 패닉하므로 hook 하나가
  둘을 모두 덮는다 — 테스트
  `a_lowering_failure_reports_through_the_same_path`가 그 사실을 고정한다.

### 결정 2: hook과 `catch_unwind`의 역할 분담

- **상황**: 둘 중 하나만으로도 뭔가는 된다. hook은 메시지와 위치를 알고,
  `catch_unwind`는 그 다음에 무엇을 할지 정할 수 있다.
- **선택과 근거**: 둘 다, 역할을 나눠서. **보고는 hook**이 한다 — 패닉이 일어난
  그 자리, 사실에 가장 가까운 곳이고, `catching` 밖으로 새는 패닉도 덮는다.
  **결정은 `catching`**이 한다 — CLI는 101로 끝내고, 서버는 그 요청 하나만
  실패시키고 세션을 유지한다. 진입점마다 "살아남는다"의 의미가 다르므로,
  그 결정은 진입점이 내려야 한다.

### 결정 3: 패닉 시 사용자 소스를 저장할 것인가

- **상황**: rustc는 ICE 때 파일을 남긴다. 재현을 위해 입력을 `/tmp`에 복사하는
  방안을 검토했다.
- **검토한 대안**:
  - A. 소스를 복사해 경로를 안내한다. 제보가 쉬워진다.
  - B. 파일 **이름**만 보고하고 공유 여부는 소유자가 정한다.
- **선택과 근거**: B. 컴파일러가 사용자 코드를 묻지 않고 다른 경로에 복사하는
  것은 개인정보 관점에서 기본값이 되어선 안 된다. 보고는 파일을 지목하고
  "공유할 수 있다면 함께 보내달라"고 말한다. 부수 효과로 `working_on`이
  소스를 붙들 필요가 없어 스레드 로컬이 `PathBuf` 하나로 끝난다.

### 결정 4: 안전망을 어떻게 테스트할 것인가

- **상황**: 그물은 무언가를 떨어뜨려 봐야 그물이다. 그런데 지금 알려진, 확실히
  패닉하는 사용자 입력이 없다.
- **검토한 대안**:
  - A. 렌더러와 `catching`만 단위 테스트한다. 진입점 배선("hook이 실제로
    설치되는가", "서버 루프가 살아남는가")은 검증되지 않는다.
  - B. 디버그 빌드에만 존재하는 단일 훅 `panic_for_test(point)` +
    `TTC_PANIC_FOR_TEST` 환경 변수.
- **선택과 근거**: B. 이 태스크의 보장 자체가 "실패했을 때의 행동"이므로 실패를
  만들 수 있어야 한다. `#[cfg(debug_assertions)]`로 릴리스 빌드에는 그 경로가
  아예 없고, 호출 지점은 세 곳(`cli`, `compile`, `server`)뿐이며 각각이 서로 다른
  보장을 검증한다. 대안 A로는 "에디터의 언어 서버가 죽지 않는다"는 이 태스크의
  핵심 보장을 끝내 확인할 수 없다.

## 작업 내역

- 2026-08-25: `ice.rs`에 `install_reporter`/`working_on`/`catching`/
  `panic_for_test`를 추가하고 모듈을 `pub mod ice`로 공개했다 — `src/server.rs`는
  바이너리 모듈이라 `crate::ice`에 닿지 못하므로 라이브러리 표면이 필요하다.
  hook은 unwinding 중에 실행되므로 그 안의 모든 접근은 실패해도 무시한다
  (`try_with`, `var_os`) — hook에서의 패닉은 프로세스 abort다.
- 2026-08-25: `main`을 `run`으로 나누고 `catching(run)`으로 감쌌다. 종료 코드는
  101(Rust 패닉의 종료 코드)로 유지한다.
- 2026-08-25: 서버 루프의 `respond` 호출을 `catching`으로 감쌌다. 실패한 요청은
  자신의 `id`를 담은 JSON 오류로 답하고 세션은 계속된다. 요청이 파싱조차 되지
  않았을 때를 위해 `request_id`가 `null`을 돌려준다.
- 2026-08-25: `compile_jobs`의 워커와 서버 `check`를 `working_on`으로 감싸
  보고가 작업 중이던 파일을 지목하게 했다. 스레드 로컬이라 `--jobs`로 병렬
  컴파일할 때도 각 워커가 자기 파일을 안다 — 병렬 실행에서도 종료 코드 101을
  확인했다.
- 2026-08-25: 검증 — CLI는 보고 후 101로 종료하고, 서버는 두 요청이 모두
  패닉해도 둘 다에 답한 뒤 stdin이 닫힐 때 0으로 끝난다.

## 이슈 및 해결

### 이슈 1: 잡힌 패닉이 작업 파일 프레임을 남긴다

- **증상**: 코드 리뷰 중 발견. `working_on`이 `push` → `work()` → `pop` 순서였는데,
  `work()`가 패닉하면 `pop`에 도달하지 못한다.
- **원인**: CLI에서는 프로세스가 끝나므로 무해하다. 그러나 이 태스크의 요점은
  **서버가 패닉에서 살아남는다**는 것이고, 살아남으면 다음 실패가 반드시 있다.
  남은 프레임 때문에 그 다음 실패가 자기와 무관한 파일을 "while compiling"으로
  지목하게 된다 — 컴파일러 버그 제보를 잘못된 곳으로 보내는 보고다.
- **해결**: `Drop` 가드로 바꿨다. 패닉 hook은 unwinding이 지역 변수를 떨어뜨리기
  **전에** 실행되므로 보고는 여전히 프레임을 보고, 그 다음 drop이 정리한다.
  `a_caught_panic_does_not_leave_its_file_behind`가 이를 고정한다.

### 이슈 2: `integration` 스위트의 간헐적 실패

- **증상**: 전체 스위트 실행 중 `std_result_and_then_on_a_variant_typed_value_
  keeps_the_chained_error` 1건 실패. 단독 실행과 `--test integration` 단독 전체
  실행(99/99)에서는 통과한다.
- **원인**: 이 변경과 무관하다. 이 테스트는 `tsc`를 띄우며, 4코어 컨테이너에서
  전체 스위트와 함께 돌 때만 실패했다 — 자원 경합으로 보인다.
- **해결**: 이 태스크에서 다루지 않는다. 재현되면 별도 태스크가 필요하다.

## 검증

toolchain 구성 후(`TTC_TSGO_ROOT`, `TTC_REQUIRE_TSGO=1`) 실행.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings` — 경고 0
- [x] `cargo test` — 전 스위트 통과, skip 없음
- [x] VS Code 확장 `server`·`typedcheck` 21건 통과(skip 0)

손으로 확인한 동작:

```
$ TTC_PANIC_FOR_TEST=compile ttc --check main.tt
error: internal compiler error: ...
  while compiling: /.../main.tt
  at: src/ice.rs:...
  ttc 0.3.0-dev.6
This is a bug in ttc, not in the code it was given. ...
$ echo $?
101
```

`--jobs 2`로 병렬 컴파일 중 워커가 패닉해도 종료 코드는 101이고, 보고는 그
워커가 맡은 파일을 지목한다. 서버는 두 요청이 연달아 패닉해도 각각 자기 `id`로
오류를 답한 뒤 stdin이 닫힐 때 0으로 끝난다.

## 결과

### 변경된 파일

- `src/ice.rs` — `install_reporter`/`working_on`/`catching`/`panic_for_test`와
  보고 렌더러, 그리고 그것들을 고정하는 6개 테스트
- `src/lib.rs` — `pub mod ice`
- `src/main.rs` — `main`/`run` 분리와 `catching`, 워커의 `working_on`
- `src/server.rs` — 요청 단위 `catching`과 `request_id`
- `tests/cli.rs` — CLI 보고와 서버 생존의 end-to-end 계약
- `docs/ai/tt.md` — ICE 동작 한 줄

### 후속

- `unwrap`/`expect` 감사: 안전망은 사고를 보고할 뿐 없애지 않는다. 255개소를
  줄이는 것은 별도 태스크로 남는다.
