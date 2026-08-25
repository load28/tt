# TASK-213: 진단 표현 계층 — 렌더러, 코드 노출, 구조화된 제안

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: `ea22022`

## 목적

tt의 진단은 `Diagnostic`에 코드·바이트 span·owner까지 이미 갖추고 있지만, 사용자가
보는 형태는 `file:line:col: message` 한 줄이다. 데이터는 있는데 표현이 없다. 더
나쁜 것은 CLI가 typed 경로에서 `code`와 `end`를 **버린다**는 점이다(`main.rs`의
`checked.diagnostics` 출력) — 같은 진단을 `--server`는 코드와 끝 위치까지 온전히
보내는데 CLI만 잃는다. 한 모델에서 나온 진단이 출구마다 다른 사실을 갖는 상태다.

이 태스크는 진단의 **표현 계층**을 세운다: 원인·부가 설명·수정 제안을 모델에
올리고, 그 모델 하나를 CLI 텍스트와 서버 와이어 포맷이 각자의 매체로 렌더한다.

## 범위

- 포함:
  - `Diagnostic`/`TtError`에 `notes`(부가 설명)와 `suggestions`(기계 적용 가능한
    수정) 추가
  - `resolve`가 이미 계산한 이름 제안을 메시지 문자열에서 구조로 승격
  - `src/render.rs` — 소스 스니펫·캐럿·note·help를 그리는 단일 렌더러
  - CLI의 untyped/typed 두 경로 모두 이 렌더러 사용, 진단 코드 출력
  - `--server` 와이어 포맷에 `notes`/`suggestions` 추가
  - `ttc explain <code>` — 규칙별 설명
- 제외:
  - ANSI 색상 (아래 결정 3)
  - 스냅샷 픽스처 전환 (TASK-215)
  - 새 진단 규칙 추가. 이 태스크는 **기존 진단의 표현만** 바꾼다.

## 의사결정

### 결정 1: 렌더러만 만들 것인가, 진단 모델을 확장할 것인가

- **상황**: 캐럿과 스니펫만 그리려면 기존 `message` 문자열로 충분하다. 그러나
  `UnknownCase`의 "did you mean" 처럼 *수정 방법*이 이미 메시지 문자열 안에
  녹아 있어, 렌더러가 그것을 `help:`로 분리해 보여줄 방법이 없다.
- **검토한 대안**:
  - A. 렌더러만 추가하고 메시지는 그대로 둔다. 작지만, 제안은 영원히 문자열에
    갇히고 에디터 quick-fix는 불가능하다. 렌더러가 문자열 모양을 보고 분리하려면
    휴리스틱이 필요한데 AGENTS.md 계약 3이 금지한다.
  - B. `Diagnostic`에 `notes`/`suggestions`를 두고, 보고 지점이 구조로 채운다.
    렌더러는 표현만 담당한다.
- **선택과 근거**: B. 계약 3("책임 있는 계층에 일반화")이 그대로 적용된다 —
  *무엇이 잘못됐고 어떻게 고치는가*는 진단 모델의 사실이고, *어떻게 보이는가*만
  렌더러의 몫이다. 같은 데이터에서 CLI의 `help:` 줄과 에디터의 CodeAction이 함께
  나온다.

### 결정 2: 제안 문구를 메시지에 남길 것인가

- **상황**: 결정 1을 따르면 `unknown-case`의 제안이 `suggestions`로 올라간다.
  메시지에 "— did you mean `Circle`?"를 남기면 CLI가 같은 말을 두 번 한다.
- **검토한 대안**:
  - A. 메시지 유지 + 제안 추가. 기존 테스트와 `docs/ai/tt.md`가 그대로지만 출력이
    중복된다. 중복을 피하려면 렌더러가 메시지 문자열을 검사해야 하는데, 이는
    계약 3이 금지하는 문자열 모양 휴리스틱이다.
  - B. 메시지는 문제만("has no case `Circel`"), 제안은 `suggestions`로. rustc가
    `error:`와 `help:`를 나누는 방식과 같다.
- **선택과 근거**: B. 중복을 구조로 없앤다. 대가는 사용자에게 보이는 문구 변경이라
  `docs/ai/tt.md`와 관련 테스트를 함께 갱신한다(AGENTS.md의 "언어 표면을 바꾸면
  `docs/ai/tt.md`도 갱신"). 제안의 가치는 산문이 아니라 **적용 가능한 편집**
  (span + 대체 문자열)이며, 그것은 B에서만 보존된다.

### 결정 3: ANSI 색상을 포함할 것인가

- **상황**: rustc 수준의 가독성에는 색상이 포함된다.
- **검토한 대안**: A. 지금 포함. B. 구조를 먼저 세우고 별도 태스크.
- **선택과 근거**: B. 색상은 렌더 결과의 모든 바이트에 영향을 주므로, 스냅샷
  게이트(TASK-215)가 자리잡기 전에 넣으면 회귀를 고정할 수단 없이 표면만 넓힌다.
  구조 → 고정 → 색상 순서가 안전하다. 후속 태스크로 등록한다.

### 결정 4: 렌더러를 어느 계층에 둘 것인가

- **상황**: CLI 전용으로 `main.rs`에 둘 수도, 라이브러리에 둘 수도 있다.
- **검토한 대안**:
  - A. `main.rs`. 이미 1935줄이고, 서버·테스트가 재사용할 수 없다.
  - B. `src/render.rs` (공개). `error.rs`의 `Display`가 이미 라이브러리에서 CLI
    형식을 렌더하고 있으므로 계층 선례가 있다.
- **선택과 근거**: B. 렌더러가 하나여야 두 출구가 드리프트하지 않고, `tests/`에서
  직접 검증할 수 있다.

### 결정 5: 렌더러의 좌표계

- **상황**: `ttc::Diagnostic`은 바이트 오프셋, `engine::Diagnostic`은 1-based
  line/column을 쓴다. 렌더러는 둘 다 받아야 한다.
- **선택과 근거**: 두 경로의 공통 분모인 **line/column**으로 렌더한다. 바이트
  경로는 이미 있는 `to_compile_error`로 변환해 들어온다. engine 경로는 변환 없이
  들어온다. 렌더러가 오프셋을 요구하면 engine 진단은 원본을 다시 파싱해야 한다.

### 결정 6: "고치는 법"을 notes와 suggestions로 나눌 것인가

- **상황**: rustc는 진단 아래에 note(사실)와 help(조언)를 따로 단다. 처음에는
  `notes: Vec<String>`과 `suggestions: Vec<Suggestion>`을 모두 두려 했다.
- **검토한 대안**:
  - A. 두 필드 모두. 표현력은 넓지만, 지금 `notes`를 채우는 보고 지점이 하나도
    없다. 죽은 필드는 clippy가 잡기 전에 이미 설계 부채다.
  - B. `suggestions` 하나로 두고 `Suggestion { message, edit: Option<Edit> }`.
    "적용 가능한 수정"과 "적용할 텍스트가 없는 조언"이 한 채널에서 구분된다.
- **선택과 근거**: B. 지금 실제로 있는 데이터는 두 종류의 *수정*뿐이다 —
  이름 오타의 대체(편집 있음)와 "빠진 암을 추가하라"(편집 없음). `notes`는 그것을
  생산하는 규칙이 생길 때 추가한다. 소비자 입장에서도 "고치는 법은 여기 하나"가
  더 단순하다.

### 결정 7: TypeScript 진단의 `ts(CODE):` 접두사

- **상황**: 렌더러가 `error[ts2322]:`를 그리는데, engine이 만드는 메시지가 이미
  `ts(2322): ...`로 시작해 같은 코드를 두 번 말하게 됐다.
- **검토한 대안**:
  - A. 메시지에 코드가 있으면 브래킷을 생략. 메시지 문자열의 모양을 보는
    분기이므로 계약 3 위반이다.
  - B. 생산 지점(`ts_message`)에서 접두사를 뺀다. 코드는 이미
    `Diagnostic::code`에 있다.
- **선택과 근거**: B. 결정 2와 같은 규칙의 세 번째 적용이다. 어떤 테스트도 이
  접두사 문구를 검증하지 않아(`grep 'ts(2'` = 0건) 회귀 위험도 없었다.

### 결정 8: 에디터의 arm 삽입 quick fix

- **상황**: 확장은 `NON_EXHAUSTIVE_RE`로 **진단 메시지를 정규식 파싱**해 빠진
  케이스를 뽑고 있었다. 그 정규식은 메시지 끝의 `(add ...)`를 앵커로 썼는데,
  결정 2로 그 문구가 suggestion으로 옮겨가면서 quick fix가 깨졌다.
- **검토한 대안**:
  - A. 정규식을 새 문구에 맞춘다. 최소 변경이지만 계약 3 위반을 유지한다.
  - B. 컴파일러가 arm 삽입 자체를 `Edit`으로 저작한다. 정규식과 확장의 arm 문자열
    조립이 함께 사라진다. 다만 `MatchAnalysis`에 body 닫는 위치를 싣고, sema에
    소스 텍스트를 넘기고, 들여쓰기를 계산하고, 케이스의 필드 이름을 그 경로까지
    가져와야 한다 — `CoveredEnum`은 이름과 origin만 갖는다.
  - C. 규칙 식별은 `code`로 바꾸고(정규식의 절반 제거), 태그 목록만 메시지의
    렌더된 리스트에서 읽는다.
- **선택과 근거**: C를 이번 태스크에, B를 후속(TASK-216)으로 분리했다. B가
  옳은 종착지이지만 네 계층을 건드리는 별도 기능이고, 이 태스크는 "기존 진단의
  표현"으로 범위를 잡았다(범위 §제외). C는 확장이 진단을 **어떤 규칙인지**
  알아내는 데 문자열을 보지 않게 만들어 위반을 절반으로 줄이고, 깨진 quick fix를
  즉시 복구한다. 남은 절반은 TASK-216에 부채로 등록했고 코드에도 그렇게 적었다.

## 작업 내역

- 2026-08-25: `./scripts/doctor` 실행 — toolchain 미설정 보고. AGENTS.md 규칙 3·4에
  따라 setup 모드를 추측하지 않고 `./scripts/setup`도 실행하지 않음. 영향은
  `tests/native.rs`와 확장 서버 테스트의 skip뿐이며 CI의 `native` job이 덮는다.
- 2026-08-25: 진단 흐름 조사. 두 개의 진단 타입이 있음을 확인:
  `crate::Diagnostic`(바이트 오프셋, tt 전용)과 `engine::Diagnostic`(line/col,
  path, `code: Option<String>`). 후자는 이미 코드와 끝 위치를 갖는데 `main.rs`의
  출력이 둘 다 버리고 있음을 확인.
- 2026-08-25: 진단 모델 확장. `Diagnostic`에 `suggestions: Vec<Suggestion>`을
  추가하고 `#[non_exhaustive]`를 붙였다(0.3.0-dev 단계에서 지금이 파괴적 변경의
  적기). `Suggestion { message, edit: Option<Edit> }`, `Edit { start, end,
  replacement }`. `TtError`에 `help()`/`suggest()` 빌더와, verify가 구조체
  리터럴로 만들던 무위치 오류를 위한 `positionless()` 생성자를 추가했다.
- 2026-08-25: `sema::report_resolution`이 `resolve`의 `UnresolvedUse.suggestion`을
  문자열에 녹이는 대신 `Edit`(이름 span → 제안 이름)으로 올리도록 바꿨다.
  메시지는 문제만 말한다. `non_exhaustive_message`에서 `(add the missing arms
  ...)`를 떼어 `NON_EXHAUSTIVE_HELP` 상수로 옮기고, untyped/typed 두 경로가 같은
  상수를 suggestion으로 붙인다.
- 2026-08-25: `engine::Diagnostic`에 `suggestions`를 추가하고, 14개 생성 지점을
  갱신했다. tt 진단을 typed 경로로 넘기는 두 지점은 그대로 통과시킨다 — 그전에는
  untyped 패스가 계산한 수정을 typed 경로가 조용히 버리고 있었다.
- 2026-08-25: `src/render.rs` 신설. `Report`(severity/code/message/path/span/
  suggestions) 하나를 헤더 + `-->` + 스니펫 + 캐럿 + `= help:`로 그린다. 한 줄
  span, 여러 줄 span의 괄호 형태, 탭 확장, 긴 span의 중간 생략, 소스 없음, 위치
  없음을 모두 다룬다. `diagnostic()`/`engine_diagnostic()`/`compile_error()` 세
  진입점으로 CLI의 세 경로가 하나의 렌더러를 쓴다.
- 2026-08-25: `Snapshot::source_of(path)` 공개. typed 경로가 진단을 그릴 때
  디스크가 아니라 **검사에 실제로 쓰인 텍스트**(overlay 포함)를 인용하도록 했다.
  블록된 파일도 덮는다 — 인용 가치가 가장 큰 파일이 정확히 그 파일이다.
- 2026-08-25: CLI 세 출력 지점(untyped compile, typed pass, blocked project)을
  렌더러로 교체. typed 경로가 버리던 `code`와 끝 위치가 이제 출력된다.
- 2026-08-25: 서버 와이어 포맷의 `check`/`typedCheck` 응답에 `suggestions`를
  추가했다. 편집의 바이트 오프셋은 진단과 같은 1-based line/column으로 변환해
  한 응답이 두 좌표계를 섞지 않는다.
- 2026-08-25: `DiagnosticCode::ALL`/`parse`/`explanation`과 `ttc explain <code>`
  추가. 설명은 `docs/ai/tt.md`의 규칙 서술에서 확인한 내용으로만 썼다. 빌드
  로그에서 복사한 `error[code]` 형태도 받는다.
- 2026-08-25: VS Code 확장 — 진단의 LSP `data`로 suggestion을 실어 보내고,
  `onCodeAction`이 편집이 있는 suggestion을 quick fix로 제공한다(컴파일러가
  저작한 수정). exhaustiveness quick fix는 `diag.code`로 식별하도록 바꿨다.
- 2026-08-25: 사용자 요청으로 typescript-go를 CI가 고정한 ref
  (`c6b013f5706d58582f566df778cc0df2683b58f5`)로 클론해 빌드하고
  `./scripts/setup --tsgo-root`로 로컬 toolchain을 구성했다. "최신 소스" 대신
  고정 ref를 쓴 이유는 CI가 그 ref에 대해서만 초록이고, API 클라이언트와
  `tsgo` 실행 파일이 버전 협상 없는 프로토콜을 쓰기 때문이다(ref 상향은
  별도 태스크).

## 이슈 및 해결

### 이슈 1: stale 버퍼의 span에서 뺄셈 언더플로

- **증상**: 렌더러 테스트 `a_span_past_the_end_of_a_stale_buffer_does_not_panic`이
  `attempt to subtract with overflow`로 패닉했다. 파일 끝을 넘는 span
  (400행짜리 진단, 6행짜리 버퍼)에서 재현된다.
- **원인**: 끝 줄을 버퍼 길이로 clamp하면서 시작 줄과의 관계를 다시 확인하지
  않았다. clamp 결과가 시작보다 **앞**으로 갈 수 있고, 여러 줄 렌더러가
  `end_line - start.line`을 계산하며 언더플로했다.
- **해결**: clamp에 시작 줄을 하한으로 추가했다
  (`end.line.min(lines.len().max(1)).max(start.line)`). 그림이 캐럿 하나로
  degrade될 뿐 패닉하지 않는다. 에디터가 편집 직후 오래된 진단을 그리는 것은
  정상 상황이라 이 경로는 반드시 총(total)이어야 한다.

### 이슈 2: 확장의 exhaustiveness quick fix가 조용히 깨짐

- **증상**: 결정 2로 메시지에서 `(add the missing arms ...)`를 떼자, 확장의
  `NON_EXHAUSTIVE_RE`가 그 문구를 앵커로 쓰고 있어 매치에 실패했다. 컴파일러
  테스트는 전부 초록이었고 확장 테스트도 이 경로를 덮지 않아 드러나지 않았다.
- **원인**: 확장이 구조화된 데이터가 아니라 **진단 문장**에서 사실을 되읽고
  있었다(계약 3 위반). 메시지 문구는 그 규칙 아래에서는 바꿀 수 없는 것이 된다.
- **해결**: 결정 8. 규칙 식별을 `diag.code`로 옮기고, 새로 만든 서버 테스트 두
  개가 이 두 quick fix를 실제 LSP 왕복으로 고정한다. 남은 태그 목록 파싱은
  TASK-216에서 컴파일러 저작 편집으로 대체한다.

### 이슈 3: `engine_cache` 테스트의 선행 실패 — toolchain 부재

- **증상**: `an_error_node_keeps_its_file_and_other_files_checkable`이 진단 2개
  대신 1개를 받아 실패.
- **원인**: 이 태스크의 변경과 무관했다. `origin/main`(e1adda8)을 별도 worktree로
  체크아웃해 그대로 실행했을 때 **동일하게 실패**했다.
- **해결**: typescript-go를 빌드해 `./scripts/setup --tsgo-root`로 toolchain을
  구성하자 통과한다. 컴파일러 변경이 아니라 환경 구성 문제였다.

### 이슈 5: CI에서만 보이는 clippy 린트 (PR #53)

- **증상**: PR #53의 `fmt / clippy / test` 잡이 실패했다.
  `src/render.rs`의 `report.span.and_then(|span| source.map(...))`가
  `manual_option_zip`으로 걸렸다. 로컬 `cargo clippy --all-targets -- -D warnings`는
  통과한 코드다.
- **원인**: 툴체인 버전 차이. 로컬 clippy 1.94.1, CI(`dtolnay/rust-toolchain@stable`)
  1.98.0. `scripts/doctor`는 MSRV 하한만 확인하므로 이 격차를 알려주지 않는다.
- **해결**: `rustup update stable`로 로컬을 1.98.0에 맞춘 뒤 재현하고,
  `report.span.zip(source)`로 고쳤다 — 린트 제안이 원래 의도한 표현이기도 하다.
  같은 툴체인에서 fmt·clippy·전 스위트를 다시 통과시켰다. 격차 자체는
  [TASK-226](./TASK-226-local-ci-toolchain-parity.md)으로 등록했다.

### 이슈 4: 확장 completion 테스트의 불안정성 (미해결, 범위 밖)

- **증상**: `completion.test.js`의 probe/멤버 완성 케이스 일부가 이 컨테이너에서
  실패한다. 이 브랜치는 1건(`a probe carries the pipeline's type through earlier
  steps`), `origin/main`은 2건(`a pipeline step's members need a probe`,
  `a match arm binding's members come from the emit`)이 실패한다.
- **원인**: 특정하지 못했다. `/tmp`의 이전 워크스페이스 1151개를 지우고
  `--test-concurrency=1`로 직렬 실행해도 트리별로 결정적이지만, ttc 바이너리만
  바꾸거나 확장 빌드만 바꿔도 **실패하는 케이스 집합이 달라진다**(네 조합이 네
  가지 결과). 즉 실패 집합이 이 diff의 함수가 아니다. 한 파일의 테스트들이 하나의
  엔진 서버 세션을 공유하는 구조와 관련된 것으로 보인다.
- **해결**: 이 태스크에서 다루지 않는다. 이 diff는 completion/probe 경로를 전혀
  건드리지 않고, 기준선이 이 테스트들을 더 많이 실패시킨다. 확장의 다른 스위트
  (`server` 17건 — 이 태스크가 추가한 quick fix 테스트 2건 포함, `typedcheck`·
  `engine`·`sidecar` 28건)는 skip 0으로 전부 통과한다. 별도 조사 태스크가 필요하다.

## 검증

toolchain 구성 후(`TTC_TSGO_ROOT=/home/user/tsgo-src`, `TTC_REQUIRE_TSGO=1`) 실행.

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings` — 경고 0
- [x] `cargo test` — 모든 스위트 통과, skip 없음
  (lib 219, cli 44, compile 332, emit_map 16, engine_cache 3, integration 99,
  native 40, passthrough 57, resolve 11, sidecar 8, stdlib 6, doc 27)
- [x] VS Code 확장: `npx tsc -b`, `server` 17건 / `typedcheck`·`engine`·
  `sidecar` 28건 전부 통과(skip 0). `completion`은 이슈 4 참조 — 기준선에서 더
  많이 실패하며 이 변경의 영향이 아니다.

## 결과

### 변경된 파일

- `src/diagnostics.rs` — `Suggestion`/`Edit`, `Diagnostic::suggestions`,
  `#[non_exhaustive]`, `DiagnosticCode::{ALL, parse, explanation}`,
  `NON_EXHAUSTIVE_HELP`
- `src/error.rs` — `TtError::{help, suggest, positionless}`
- `src/render.rs` (신규) — 렌더러와 세 진입점
- `src/sema.rs` — 이름 제안을 `Edit`으로, exhaustiveness 조언을 `help`로
- `src/engine/semantics.rs` — `Diagnostic::suggestions`, `ts_message`에서 코드
  접두사 제거
- `src/engine/snapshot.rs` — `Snapshot::source_of`
- `src/main.rs` — 세 출력 지점을 렌더러로, `ttc explain`
- `src/server.rs` — 와이어 포맷의 `suggestions`
- `src/lib.rs`, `src/verify.rs` — 재export와 생성자 사용
- `editors/vscode/server/src/{ttc,server}.ts` — suggestion 전달과 quick fix
- `tests/{cli,compile,integration,native}.rs`,
  `editors/vscode/server/src/test/server.test.ts` — 새 계약 테스트와 형식 갱신
- `docs/ai/tt.md`, `website/src/essay.json` — 사용자에게 보이는 형식 갱신

### 후속

- [TASK-216](./TASK-216-compiler-authored-arm-edits.md) — arm 삽입을 컴파일러가
  저작하는 편집으로 (결정 8)
- [TASK-220](./TASK-220-diagnostic-colour.md) — 진단 렌더러의 ANSI 색상 (결정 3).
  선행 조건이던 스냅샷 게이트는 TASK-215로 갖춰졌다
- [TASK-217](./TASK-217-completion-test-instability.md) — 확장 `completion`
  테스트 불안정성 조사 (이슈 4)
