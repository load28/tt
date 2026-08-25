# TASK-217: VS Code 확장 completion 테스트의 불안정성 조사

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: (아래 "작업 내역")

## 목적

`editors/vscode/server/src/test/completion.test.ts`의 probe/멤버 완성 케이스가
로컬 컨테이너에서 결정적으로 실패하는데, **어떤 케이스가 실패하는지가 실행
환경에 따라 달라진다**. TASK-215 작업 중 확인한 내용:

| 확장 빌드 | `ttc` 바이너리 | 실패 케이스 |
|---|---|---|
| 이 브랜치 | 이 브랜치 | `a probe carries the pipeline's type through earlier steps` |
| 이 브랜치 | `origin/main` | `a match arm binding's members come from the emit` |
| `origin/main` | 이 브랜치 | 위 두 개 |
| `origin/main` | `origin/main` | `a pipeline step's members need a probe`, `a match arm binding's...` |

이 상태로는 이 스위트가 회귀를 잡는지 아무도 신뢰할 수 없다.

## 범위

- 포함:
  - 실패의 실제 원인 특정 — 한 파일의 테스트들이 **엔진 서버 세션 하나를
    공유**하는 구조가 첫 용의자다.
  - 원인이 테스트 격리라면 케이스마다 세션을 분리하거나 워크스페이스를 격리한다.
  - 원인이 컴파일러/엔진이라면 해당 계층에 회귀 테스트를 만들고 고친다.
- 제외: 실패를 `skip`으로 덮는 것.

## 의사결정

### 0. 첫 용의자는 무죄였다

세션 공유는 원인이 아니다. 케이스마다 `mkdtemp`로 워크스페이스를 만들고,
프로젝트 정체성은 `(tsconfig, root)`인데 tsconfig가 없으면 root는 **그 파일의
디렉터리**다(`engine::identity_of`). 즉 케이스마다 다른 `Project`이고 서비스
세션도 따로다. 실제 원인은 컴파일러 쪽 버그 **두 개**였고, 둘 다 "환경에 따라
답이 달라지는" 성질을 갖고 있어서 실패 집합이 움직였다.

### 1. `@tt/runtime` materialize를 "파싱된 텍스트"에 걸어 두면 안 된다

`ensure_runtime_module`은 `serve_one`에서 `scan_module_with_kind(text).uses_pipeline`
일 때만 호출됐다. 그 플래그는 **파싱된 AST**를 걷는다(`program_uses_pipeline`).

에디터가 가장 어려운 질문을 하는 순간은 사용자가 `.`을 **막 입력한** 순간이고,
그때 버퍼는 파싱되지 않는다:

```tt
const out = r
  |> Result.mapP((n) => n + 1)
  |> .          // 반쪽 step — stray-pipe
```

파싱이 안 되므로 `uses_pipeline == false` → `node_modules/@tt/runtime`이
만들어지지 않는다. 그런데 probe는 버퍼를 **기워서** 묻고, 기운 형태는 파싱되어
`$tt_ap`를 방출한다. 그 import가 해석되지 않으니 식 전체가 타입 없이 남고
멤버 목록이 **비어서** 돌아온다.

`@tt/std`는 이미 세션 시작 시 무조건 쓰고 있었다. 이 비대칭이 버그의 전부이므로,
runtime도 같은 자리로 옮겼다. "이 파일이 무엇을 필요로 하는가"는 파싱이 답하는
질문이고, 파싱이 실패하는 순간이 바로 에디터가 답해야 하는 순간이다.

재현(고치기 전, 결정적):

```
MEMBER: {"items":[],"member":true,"probe":1}
materialized: std
```

고친 뒤:

```
MEMBER: {"items":[{"kind":"property","label":"kind",...}],"member":true,"probe":1}
materialized: runtime, std
```

### 2. LSP 바이너리 경로는 절대 경로여야 한다

`service_binary`는 `../typescript-go/built/local/tsgo`(형제 체크아웃)를
**상대 경로 그대로** 돌려줬다. 서비스는 `current_dir(root)` — 즉 **프로젝트
디렉터리** — 에서 spawn되므로, 그 상대 경로는 측정된 적 없는 디렉터리를 기준으로
해석된다. tt 저장소 안에서 열린 프로젝트는 되고 임시 디렉터리의 프로젝트는
안 되는, 프로젝트별로 갈리는 실패가 된다.

컴파일러 백엔드는 이미 같은 문제를 `Toolchain::check`에서 해결해 두었다
("a relative path here would resolve against neither"). 서비스 경로만 그 규칙을
못 받았을 뿐이다. 네 갈래 모두 절대 경로로 통일했다.

### 3. 가드는 컴파일러의 해석 규칙을 **전부** 흉내 내야 한다

확장 테스트의 `findTsgo`는 `TTC_TSGO_ROOT`와 node_modules 플랫폼 패키지만
봤다. ttc는 `TTC_TSGO_BIN`과 형제 체크아웃도 본다. 규칙의 절반만 흉내 내면
가드와 컴파일러가 **양방향으로** 어긋난다.

- 가드는 "없다"고 skip하는데 ttc는 찾는다 → 돌 수 있었던 케이스가 안 돈다.
- 가드는 "있다"는데 ttc는 못 찾는다 → 케이스가 실패한다.

이것이 "실패 집합이 환경에 따라 달라진다"의 나머지 절반이다. `findTsgo`를
`service_binary`의 순서대로 맞췄다.

### 4. 가드 없는 typed 케이스에 가드를 준다

`typedcheck.test.ts`는 `compilerAvailable()`만 보고 있었는데, 그중 두
케이스는 진짜 타입 답을 요구한다. TypeScript 7이 없는 기계에서 그 둘은
skip이 아니라 **fail**했다 — "도구가 없다"와 "기능이 깨졌다"를 구분하지
못하는 상태다. `skipTyped`를 만들어 나눴다. 이것은 실패를 덮는 skip이 아니라
전제 조건을 선언하는 skip이고, CI의 native 잡은 `TTC_REQUIRE_TSGO`와
`grep -q "^# skipped 0$"`로 그 skip이 CI에서는 절대 발동하지 않음을 계속
보장한다.

## 작업 내역

1. `src/engine/language.rs`: 세션 시작 시 `ensure_std_module`과
   `ensure_runtime_module`을 함께 호출. `serve_one`의 조건부 호출과 이제
   쓰이지 않는 `root` 인자를 제거. 단위 테스트를 새 계약으로 갱신하고
   "이미 있는 것을 덮어쓰지 않는다"까지 고정.
2. `src/typescript/service.rs`: `service_binary`의 모든 갈래를 절대 경로로.
3. `tests/native.rs`: 컴파일러 계층 회귀 테스트
   `a_probe_answers_in_a_pipeline_the_buffer_cannot_parse_yet` — 파싱되지 않는
   버퍼에서 probe가 `Result`의 멤버를 답하는지.
4. `editors/vscode/server/src/ttc.ts`: `findTsgo`를 `service_binary`의 순서에
   맞춤(`TTC_TSGO_BIN`, 형제 체크아웃 추가).
5. `editors/vscode/server/src/test/typedcheck.test.ts`: `skipTyped` 도입.

## 이슈 및 해결

- **증상**: 새로 넣은 Rust 회귀 테스트가
  `cannot run ../typescript-go/built/local/tsgo: No such file or directory`로 실패.
- **원인**: 결정 2의 버그. 테스트가 임시 디렉터리에 프로젝트를 만들고 엔진을
  in-process로 돌리므로, 상대 경로 해석이 처음으로 드러났다.
- **해결**: 결정 2. 테스트를 고치지 않고 컴파일러를 고쳤다 — 테스트가 옳았다.

- **증상**: 환경 변수를 테스트 안에서 세팅하려다 `unsafe_code = "forbid"`에 걸림.
- **해결**: 세팅할 필요가 없었다. 가드(`require_tsgo!`)는 ttc 자신의 해석 규칙을
  흉내 내므로, 통과했다는 것은 **컴파일러가 스스로 찾았다**는 뜻이다. 테스트가
  경로를 알려주고 통과하는 것보다 강한 계약이다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 전체 초록 (native 41)
- [x] 확장 스위트, `TTC_TSGO_ROOT` 설정: **88/88 pass, skip 0** (3회 반복)
- [x] 확장 스위트, 툴체인 없음: **0 fail, 51 skip** — 이전에는 가드 없는 2건이
      실패했다
- [x] `completion.test.js` 단독 3회 반복 7/7

## 결과

실패 집합이 환경의 함수가 아니게 됐다. 툴체인이 있으면 전부 돌고 전부 통과하며,
없으면 전부 skip하고 아무것도 실패하지 않는다. 두 상태 모두 사실을 말한다.

조사가 찾은 것은 테스트의 문제가 아니라 제품의 문제였다 — 사용자가 `.`을 막
입력한 순간, 즉 완성 기능이 가장 필요한 순간에 파이프라인 안에서 멤버 목록이
비는 버그였고, 그것을 잡은 것이 이 스위트다.

### 변경 파일

- `src/engine/language.rs`
- `src/typescript/service.rs`
- `tests/native.rs`
- `editors/vscode/server/src/ttc.ts`
- `editors/vscode/server/src/test/typedcheck.test.ts`
- `docs/tasks/INDEX.md`
