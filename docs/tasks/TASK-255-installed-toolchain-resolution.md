# TASK-255: 설치된 패키지만으로 동작하는 에디터 툴체인 해석

- **상태**: 완료
- **시작일**: 2026-08-27
- **완료일**: 2026-08-27
- **커밋**: `87fb962`

## 목적

Nightly npm 패키지와 Nightly VSIX만 설치한 개발자의 에디터에서 진단·타입
추론·자동완성이 전부 동작하지 않았다. 같은 프로젝트에서 컴파일(`npx ttc`)은
정상이었다. 원인은 "이 프로젝트의 ttc와 TypeScript가 무엇인가"를 답하는
해석 규칙이 **TT 저장소를 직접 빌드한 환경만** 알고 있었다는 것이다. TT를
빌드하지 않는 소비자 환경이 1급 경로가 되도록 해석 계층을 고친다.

## 범위

- 포함
  - `ttc`가 TypeScript 7을 찾는 **순서와 레이아웃**을 한 모듈로 모으고
    두 소비자(체커의 API 클라이언트, 언어 서비스의 실행 파일)가 그것을
    읽게 한다. 실행 파일 이름을 실제 npm 배포 레이아웃과 일치시킨다.
  - 그 순서는 "환경 변수로 지목한 tsgo → 직접 빌드한 체크아웃 → 프로젝트가
    설치한 TypeScript 7 패키지"로 명시한다.
  - 확장의 컴파일러 해석 사다리에 "프로젝트가 설치한 `@load28/tt-lang`"을
    넣는다. 그 답은 패키지 자신(`binaryPath()`)이 낸다.
  - 엔진 세션이 `tt.compilerPath`를 반영하기 전에 영구 침묵하는 문제를
    고친다.
  - 확장 테스트의 툴체인 가드가 서버와 같은 해석을 쓰게 한다.
- 제외
  - tsgo를 직접 빌드한 개발자를 위한 새 설정 키. 환경 변수 경로는 사용자가
    감수하기로 한 조건이고, npm에서 TypeScript 7을 설치하면 환경 변수 없이
    동작하게 되므로 새 표면을 늘리지 않는다.
  - `create-tt` 스캐폴드의 의존성 구성(어느 TypeScript 7 배포를 기본으로
    권할지는 별개 결정).

## 의사결정

### 결정 1: 설치된 TypeScript 패키지의 레이아웃을 한 곳에서 기술한다

- **상황**: `--check-types`는 동작하는데 호버·완성·정의 이동만 전부
  침묵했다. 체커 경로(`native.rs`)는 패키지의 **API 클라이언트**
  (`dist/api/sync/api.js`)를 찾고, 언어 서비스 경로(`service.rs`)는 같은
  패키지의 **실행 파일**을 따로 찾는데, 후자가 `lib/tsc`로 하드코딩되어
  있었다. 실제 배포물은 `@typescript/native-preview-<platform>/lib/tsgo`다.
- **검토한 대안**
  - A. `service.rs`의 파일명만 `tsgo`로 바꾼다. 최소 수정이지만 같은
    레이아웃 지식이 여전히 두 곳에 흩어져 있어 다음 배포 변경에서 또
    갈라진다. Windows `.exe`도 여전히 빠져 있다.
  - B. 설치된 배포판의 레이아웃(클라이언트 패키지, 플랫폼 패키지,
    실행 파일 이름)을 한 모듈로 모으고 두 소비자가 같은 기술을 읽는다.
- **선택과 근거**: B. 두 반쪽은 **한 빌드에서 나온 한 쌍**이라는 것이
  이미 `native.rs`의 계약이므로, 그 쌍을 어디서 찾는지도 한 곳에서
  말해야 한다. 새 `src/typescript/toolchain.rs`가 업스트림
  `getExePath.js`의 규칙(`typescript`는 `tsc`, 그 외 배포는 `tsgo`)을
  그대로 담고, `native.rs`와 `service.rs`가 이를 읽는다. 반쪽만 맞는
  복사본이 만드는 실패 모드 — CLI는 타입 검사되는데 에디터는 아무것도
  답하지 않는 상태 — 가 구조적으로 불가능해진다.

### 결정 1-1: 레이아웃뿐 아니라 **탐색 순서**도 그 모듈이 갖는다

- **상황**: 결정 1로 레이아웃을 합치고 나니, 같은 순서를 두 함수가 여전히
  따로 적고 있었다. 게다가 실패 처리가 서로 달랐다 — `native.rs`는
  `TTC_TSGO_ROOT`가 가리킨 트리가 비었으면 그것을 에러로 보고했고,
  `service.rs`는 조용히 다음 후보로 넘어갔다. "환경 변수로 지목한 tsgo가
  npm 패키지보다 우선"이라는 요구사항은 두 반쪽에서 **같은 뜻**이어야 한다.
- **검토한 대안**
  - A. 각 함수에 순서를 그대로 두고 주석으로 "같아야 한다"고 적는다.
    지금 고치고 있는 결함이 정확히 그 방식의 결과다.
  - B. 순서를 `Source` 목록 하나로 만들고 두 반쪽이 그것을 순회한다.
- **선택과 근거**: B. `toolchain::sources()`가 `Named → Checkout(지목) →
  Checkout(형제) → Installed`를 돌려주고, `client()`(API 클라이언트)와
  `service_binary()`(언어 서버)가 각자 자기 반쪽을 그 순서대로 찾는다.
  실패 처리도 하나로 정리했다: **지목한 것(1·2)이 없으면 에러로 보고하고
  멈춘다**. 사용자가 가리킨 툴체인 대신 조용히 다른 TypeScript를 쓰는
  것은 "지목"이라는 말과 모순이고, ttc가 이미 문서화한 계약("빌드되지 않은
  트리는 짐작하지 않고 그대로 보고한다")이기도 하다. `../typescript-go`
  형제 디렉터리는 지목이 아니라 관례이므로 없으면 건너뛴다.

### 결정 2: 프로젝트의 컴파일러는 설치된 패키지에게 묻는다

- **상황**: 확장의 해석 순서는 `tt.compilerPath` → 워크스페이스의
  `target/{release,debug}/ttc` → `file:`로 설치된 개발용 패키지 →
  PATH의 `ttc`였다. npm에서 설치한 `@load28/tt-lang`은 어디에도 없어서
  소비자 프로젝트는 항상 PATH로 떨어지고, 거기에 ttc가 없으면 확장이
  컴파일러 없이 돌았다.
- **검토한 대안**
  - A. 확장이 `node_modules/@load28/tt-lang-<platform>/bin/ttc` 경로를
    직접 조립한다. pnpm·yarn처럼 호이스팅하지 않는 레이아웃에서 깨지고,
    `platforms.json`의 플랫폼 표를 확장이 다시 갖게 된다.
  - B. `node_modules/.bin/ttc` 런처를 띄운다. node 프로세스가 한 겹 더
    끼고 Windows에서는 `.CMD`라 셸이 필요하다.
  - C. 설치된 패키지의 `index.js`가 공개한 `binaryPath()`에게 묻는다.
- **선택과 근거**: C. `npm/tt-lang/index.js`는 자신이 "`ttc` 런처,
  unplugin, **에디터 서버**"를 위해 바이너리를 찾아 주는 모듈이라고
  이미 선언하고 있다. 그 계약을 소비하면 `TTC_BINARY` 우선순위,
  `file:` 개발 설치, 플랫폼 패키지 해석이 전부 한 벌로 따라오고,
  `require.resolve`가 npm 호이스팅·pnpm 스토어·`file:` 링크를 알아서
  처리한다. 부수 효과로 `dev.ts`의 `devPackageCompiler`(개발 설치 전용
  중복 구현)를 지웠다 — 개발 설치는 이제 일반 규칙의 한 경우다.

### 결정 3: 엔진 세션의 실패 카운트를 컴파일러별로 센다

- **상황**: `tt.compilerPath`를 정확히 지정해도 에디터가 살아나지 않는다는
  보고가 있었다. 엔진은 두 번 연속 기동 실패하면 "이 ttc에는 `--server`가
  없다"로 보고 프로세스 수명 동안 자신을 끈다. 그런데 그 카운터는 전역이고,
  문서 동기화 경로(`didOpen`/`didChange`)는 설정을 await할 수 없어
  `findCompiler("")`로 해석한 값을 쓴다. 그래서 설정이 도착하기 전에 잘못된
  컴파일러로 두 번 실패하면, 그 뒤 올바른 경로가 들어와도 엔진은 영영
  깨어나지 않았다.
- **검토한 대안**
  - A. 카운터를 없앤다. `--server`가 없는 구 ttc에서 키 입력마다 프로세스를
    띄우게 된다 — 카운터가 존재하는 이유를 되돌리는 셈.
  - B. 카운터를 "그 컴파일러에 대한" 판정으로 만들고, 설정·감시 파일 변경
    같은 명시적 환경 변화에서 다시 무장한다. 더불어 `initialized`에서
    설정을 한 번 당겨 와 문서가 도착하기 전에 해석값을 확정한다.
- **선택과 근거**: B. 실패 판정의 주어는 "엔진"이 아니라 "이 ttc"다.
  다른 경로의 ttc는 아무것도 실패한 적이 없으므로 판정을 물려받지
  않는다. `lsp-architecture.md` §50-4가 약속한 "`tt.compilerPath` 변경이
  세션에 반영된다"가 이 변경으로 실제로 성립한다.

### 결정 4: 테스트 가드가 서버와 같은 해석을 쓴다

- **상황**: 확장 테스트는 `COMPILER = "ttc"`(PATH 이름)로 고정되어 있어
  개발 환경에서 97개 중 65개가 "ttc not on PATH"로 스킵됐다. 이번 결함이
  정확히 그 스킵 뒤에 숨어 있었다 — 소비자 해석 경로를 아무도 실행하지
  않았다.
- **선택과 근거**: 가드를 `findCompiler("", [repoRoot, cwd])`로 바꿔 서버와
  같은 규칙으로 컴파일러를 찾게 했다. TASK-217이 tsgo 가드에 대해 내린 것과
  같은 결론이다: 가드가 서버 규칙의 일부만 알면, 스킵이 "도구가 없다"가
  아니라 "우리가 못 찾았다"를 뜻하게 된다.

## 작업 내역

- 2026-08-27: 재현. npm에 게시된 nightly(`@load28/tt-lang@next`,
  0.4.0-dev.20260827.226)를 빈 프로젝트에 설치하고 `.tt` 파일을 만들었다.
  - `ttc --check` / `--check-types`: 정상(exit 0).
  - `ttc --server`의 `hover`/`completion`: `no tsgo language server found`.
    같은 프로젝트에 `@typescript/native-preview`가 설치돼 있고 `typedCheck`는
    통과하는 상태였다 — 두 해석 규칙이 갈라져 있다는 증거.
  - 확장의 `findCompiler("", [project])` → `"ttc"`. 그 프로젝트에는 PATH에
    ttc가 없다(패키지의 `.bin`에만 있다).
- 2026-08-27: `@typescript/native-preview` 배포물을 열어 레이아웃 확인.
  실행 파일은 `lib/tsgo`이고, `lib/getExePath.js`가 "`typescript` 패키지는
  `tsc`, 그 외는 `tsgo`" 규칙을 갖고 있다. `service.rs`는 두 배포 모두
  `lib/tsc`로 찾고 있었다.
- 2026-08-27: `src/typescript/toolchain.rs` 신규. 배포판 표(클라이언트
  패키지 / 플랫폼 패키지 base / 실행 파일 이름), 탐색 순서(`sources()`),
  두 반쪽의 해석(`client()` / `service_binary()`), `node_modules` 상향
  탐색, `os_name`/`arch_name`을 모두 이 모듈로 모았다. `native.rs`의
  `Toolchain`은 `toolchain::Client`가 되고, `service.rs`는
  `service_binary`를 재수출만 한다. 두 곳에 중복돼 있던
  `env_path`/`absolute`/`os_name`/`arch_name`을 제거했다.
- 2026-08-27: `editors/vscode/server/src/install.ts` 신규 —
  워크스페이스 루트에서 상향 탐색한 `@load28/tt-lang`의 `binaryPath()`에게
  묻는다. `ttc.ts::findCompiler`의 3순위로 넣고, `dev.ts`의
  `devPackageCompiler`를 제거했다. `findTsgo` 가드의 실행 파일 이름도
  배포 레이아웃에 맞췄다(+ Windows `.exe`).
- 2026-08-27: `engine.ts`의 실패 카운트를 컴파일러별로 바꾸고
  `retryEngineServer()`를 추가. `server.ts`는 `initialized`와 설정·감시 파일
  변경에서 `refreshCompiler()`로 해석값을 확정한다.
- 2026-08-27: 검증. 아래 "검증" 절.

## 이슈 및 해결

### 이슈 1: 컴파일은 되는데 에디터의 모든 타입 기능이 침묵

- **증상**: nightly 패키지 + nightly VSIX만 설치한 프로젝트에서 진단·호버·
  자동완성·정의 이동이 전부 없음. `npx ttc`는 정상.
- **원인**: 두 개의 독립된 결함이 겹쳤다.
  1. 확장이 프로젝트가 설치한 ttc를 찾지 못하고 PATH의 `ttc`로 떨어졌다
     (해석 사다리에 소비자 설치가 없음).
  2. ttc가 언어 서비스용 tsgo를 `@typescript/native-preview-<platform>/lib/tsc`
     에서 찾았다. 실제 파일명은 `lib/tsgo`. 그래서 `tt.compilerPath`를 직접
     지정해 1을 우회해도 타입 계층은 여전히 침묵했다.
- **해결**: 결정 1·2. 각 계층이 자기 책임으로 고쳤다 — 배포 레이아웃은
  컴파일러의 TypeScript 어댑터가, 프로젝트의 컴파일러는 설치된 패키지가
  답한다.

### 이슈 2: `tt.compilerPath`를 지정해도 살아나지 않음

- **증상**: 컴파일러 경로를 정확히 지정한 뒤에도 에디터 기능이 돌아오지
  않음.
- **원인**: 전역 실패 카운터. 설정이 도착하기 전 문서 동기화가 잘못된
  컴파일러로 엔진을 두 번 띄우려다 실패하면 엔진이 프로세스 수명 동안
  꺼졌고, 이후 올바른 경로에도 반응하지 않았다.
- **해결**: 결정 3.

### 이슈 3: 확장 테스트가 결함을 덮고 있었음

- **증상**: 97개 중 65개가 스킵되며 통과.
- **원인**: 가드가 PATH의 `ttc`만 알았다.
- **해결**: 결정 4. 같은 환경에서 스킵이 26개로 줄고 71개가 실제로 실행된다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 유닛·통합 전부 통과(신규 `typescript::toolchain` 4건 포함)
- [x] `./scripts/ci` — agents·rust·npm·native·extension 전 단계 통과
- [x] `npm test`(editors/vscode) — 99개 중 90 pass / 0 fail / 9 skip
      (변경 전 같은 환경에서 97개 중 32 pass / 65 skip)
- [x] 회귀 테스트가 결함을 실제로 고정하는지 확인: `engine.ts`의
      컴파일러별 판정을 되돌리면 `session.test.ts`의 첫 케이스가 실패한다
- [x] 소비자 프로젝트 E2E: npm 설치만 한 프로젝트(`@load28/tt-lang`,
      `@typescript/native-preview`)에서 실제 language server를 stdio로 띄워
      환경 변수 없이 hover(`const n: number`)·definition·completion(1067건)·
      signature help·diagnostics가 모두 응답하는 것을 확인.
- [x] 툴체인 우선순위 E2E(`ttc --server`에 hover/completion/typedCheck를
      직접 물어 확인):
      1. `TTC_TSGO_ROOT`가 빌드된 체크아웃을 가리키면 그쪽이 쓰인다 —
         프로젝트의 TypeScript 7을 지워도 계속 답한다.
      2. 환경 변수가 없으면 프로젝트가 설치한 TypeScript 7이 쓰인다.
      3. `TTC_TSGO_ROOT`가 빌드되지 않은 트리를 가리키면 **두 반쪽이 같은
         문장으로** 무엇이 없는지 보고한다 (패키지로 조용히 넘어가지 않음).
      4. 아무것도 없으면 두 반쪽이 각자의 설치 안내를 낸다.

## 결과

변경 파일:

- `src/typescript/toolchain.rs` (신규 — 순서·레이아웃·해석),
  `src/typescript/mod.rs`, `src/typescript/native.rs`(`Toolchain` 이동),
  `src/typescript/service.rs`(`service_binary` 이동·재수출)
- `editors/vscode/server/src/install.ts` (신규),
  `editors/vscode/server/src/ttc.ts`, `editors/vscode/server/src/dev.ts`,
  `editors/vscode/server/src/engine.ts`, `editors/vscode/server/src/server.ts`
- `editors/vscode/server/src/test/install.test.ts` (신규),
  `editors/vscode/server/src/test/session.test.ts` (신규),
  `editors/vscode/server/src/test/dev.test.ts`,
  `editors/vscode/server/src/test/toolchain.ts`, 나머지 테스트의 skip 문구
- `editors/vscode/README.md`, `editors/vscode/package.json`,
  `docs/design/lsp-architecture.md`

결과: TT 저장소를 빌드하지 않은 개발자가 `@load28/tt-lang`과 TypeScript 7을
설치하고 VSIX를 깔면, 설정도 환경 변수도 없이 에디터의 모든 계층이 동작한다.
직접 빌드한 tsgo를 쓰는 경우는 기존대로 `TTC_TSGO_ROOT`/`TTC_TSGO_BIN`이
그대로 유효하다.
