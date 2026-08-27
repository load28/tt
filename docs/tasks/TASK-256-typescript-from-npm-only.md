# TASK-256: TypeScript는 프로젝트가 설치한 npm 패키지 하나로

- **상태**: 완료
- **시작일**: 2026-08-27
- **완료일**: 2026-08-27
- **커밋**: —

## 목적

TASK-255에서 "설치된 TypeScript 패키지"를 해석 경로에 넣고 나니, 같은 질문에
답하는 경로가 셋이 되었다: 환경 변수(`TTC_TSGO_*`), 직접 빌드한 typescript-go
체크아웃, 그리고 프로젝트가 설치한 패키지. 하나의 사실에 세 개의 출처가 있으면
에디터와 CLI가 서로 다른 TypeScript를 쓸 수 있고, 실제로 그것이 TASK-255가 고친
장애의 형태였다. 출처를 **프로젝트가 설치한 패키지 하나**로 줄인다.

## 범위

- 포함
  - `TTC_TSGO_API`/`TTC_TSGO_BIN`/`TTC_TSGO_ROOT` 해석과 `../typescript-go`
    형제 탐색을 컴파일러에서 삭제한다.
  - `.tt-dev/toolchain.json`, `scripts/setup`의 `--tsgo-root`/`--tsgo-npm`
    두 모드, 확장의 `dev.ts` 환경 주입 계층, npm 런처의 toolchain 주입을
    삭제한다.
  - 이 저장소가 쓰는 TypeScript를 루트 `package.json`이 **정확한 버전으로**
    고정하고, CI의 typescript-go 체크아웃·Go 빌드 단계를 `npm ci`로 바꾼다.
  - 테스트 가드와 테스트 프로젝트의 위치를 새 규칙에 맞춘다.
  - AGENTS.md·CONTRIBUTING·README(영·한)·설치 가이드(영·한)·확장 README·
    npm README·홈페이지 콘텐츠를 갱신한다.
  - 저장소가 쓰는 버전과 사용자에게 안내하는 스펙을 각각 정하고, 그 둘이
    임의로 어긋나지 못하게 고정한다.
- 제외
  - 선언 방출을 실행 파일(`--emitDeclarationOnly`) 경로로 옮기는 작업. 아래
    결정 2 참고 — 필요 없어졌다.

## 의사결정

### 결정 1: TypeScript의 출처를 하나로 줄인다

- **상황**: TASK-255 직후의 해석 순서는 "환경 변수 → 체크아웃 → 설치된 패키지"
  였다. 세 경로 모두 같은 질문("이 프로젝트의 TypeScript는 무엇인가")에 답한다.
- **검토한 대안**
  - A. 순서를 문서화하고 셋을 유지한다. 우선순위가 명확해도, 셸이 내보낸
    변수를 에디터가 상속하는지 여부에 따라 에디터와 빌드가 갈라지는 경로가
    남는다 — TASK-255에서 실제로 겪은 형태다.
  - B. 프로젝트가 설치한 패키지만 남긴다.
- **선택과 근거**: B. 확장은 TypeScript를 직접 찾지 않고 프로젝트의 ttc를
  띄우며, 그 ttc가 프로젝트의 `node_modules`를 본다. 덮어쓸 수단이 없어지면
  "에디터와 CLI가 같은 TypeScript를 쓴다"가 문서상의 약속이 아니라 **구조적
  사실**이 된다. 우회로가 없으므로 진단도 하나뿐이다: 없으면 설치하라고 한다.

### 결정 2: 선언 방출 때문에 체크아웃이 필요하다는 전제를 재검증했다

- **상황**: 체크아웃이 남아 있던 유일한 이유는 `ttc --types`(사이드카)였다.
  CI 주석도 "언어 서버와 선언 방출까지 돌려야 하므로 npm 미리보기 패키지로는
  부족하다"고 적고 있었다.
- **검토한 대안**
  - A. 전제를 받아들이고 체크아웃을 "선언 방출 전용"으로 남긴다.
  - B. 선언 방출을 실행 파일(`tsgo --emitDeclarationOnly`) 경로로 옮긴다.
    측정해 보니 60개 파일 프로젝트에서 인프로세스 API 800ms 대 실행 파일
    1,400ms — 별도 프로세스가 lib부터 프로그램 전체를 다시 읽기 때문이고,
    프로젝트가 클수록 격차가 커진다.
  - C. 전제를 다시 확인한다.
- **선택과 근거**: C. 전제가 틀렸다. `@typescript/native-preview`는 2026-07-07에
  멈춘 **구 미리보기 채널**이고, TypeScript 7은 이미 정식 `typescript` 패키지로
  나와 있다(`latest` 7.0.2, `next` 7.1.0-dev). 7.1의 API 클라이언트에는
  `getDeclarationEmit`이 있고, 실제로 `ttc --types`가 `.tt.d.ts`와 맵까지
  정상 생성한다. 즉 **체크아웃이 필요한 이유는 남아 있지 않고**, B의 성능
  대가도 치를 이유가 없다. CI 주석의 "부족하다"는 절반은 TASK-255가 고친
  `lib/tsc`/`lib/tsgo` 결함 때문이었고, 나머지 절반은 낡은 채널을 보고 있었기
  때문이다.

### 결정 3: 저장소는 나이틀리를 태그가 아니라 정확한 버전으로 고정한다

- **상황**: 선언 방출 API는 7.1 라인에 있고 7.1은 아직 정식 릴리스가 아니다.
- **검토한 대안**
  - A. `typescript@7`(정식 7.0.2) 기준. `--types`와 사이드카가 동작하지 않는다.
  - B. `typescript@next` 태그. 항상 최신이지만, 어제 통과한 CI가 오늘 업스트림
    변경으로 깨질 수 있다.
  - C. 정확한 나이틀리 버전 고정 (`7.1.0-dev.20260826.1`).
- **선택과 근거**: C. 지금 CI가 typescript-go 커밋을 고정해 얻던 재현성을 훨씬
  단순한 수단으로 유지한다. 7.1이 정식 출시되면 핀을 그쪽으로 올리는 것으로
  끝나고, 그때 이 결정은 흔적 없이 사라진다. 소비자에게는 7.0으로도 컴파일·타입
  검사·에디터가 전부 동작하고, 사이드카만 7.1을 요구한다 — 그 사실을 컴파일러가
  직접 문장으로 말한다.

### 결정 3-1: 저장소는 정확한 나이틀리, 공개 안내는 `next` 태그

- **상황**: 결정 3으로 저장소는 나이틀리를 고정했는데 문서는 `typescript@7`을
  안내하고 있었다. npm 범위는 프리릴리스를 매칭하지 않으므로 그 명령은 7.0.2를
  설치하고, 사용자는 사이드카를 못 쓴다. 저장소가 검증한 것과 사용자가 받는
  것이 다르다 — 이 태스크가 없애려던 바로 그 형태다.
- **검토한 대안**
  - A. 문서에 "사이드카를 쓰려면 7.1 나이틀리를 설치하라"는 각주를 단다.
    각주는 읽히지 않고, `create-tt`은 여전히 TypeScript를 설치하지 않는다.
  - B. 모든 곳이 저장소의 정확한 핀을 그대로 말한다. 하루만 지나면 문서가
    낡고, 매일 밤 일곱 개 문서를 올릴 사람은 없다.
  - C. 대상에 따라 나눈다 — 저장소는 정확한 나이틀리, 공개 안내는 `next`.
- **선택과 근거**: C. 두 대상이 원하는 것이 실제로 다르다. CI는 지난주와 같은
  프로그램을 비교해야 하므로 움직이는 버전을 쓸 수 없고, README는 오늘 쓴
  나이틀리가 내일 낡는다. `next`는 7.1 라인을 가리키면서 낡지 않는다.
  `create-tt`이 만드는 프로젝트도 사용자의 프로젝트이므로 `next`를 넣는다 —
  `"typescript": "next"`가 package.json에서 정상 해석되는 것을 확인했다.
  **둘 다 피해야 하는 것은 `typescript@7`이다**: 7.0 라인으로 해석되고,
  그 API 클라이언트는 선언 방출을 못 하며, 어떤 경고도 없이 사이드카만
  조용히 사라진다.
  차이를 다음 사람이 임의로 좁히지 못하도록 `npm/scripts/typescript-version.test.mjs`가
  양쪽을 고정한다 — 저장소 핀이 정확한 버전인지, 설치된 것이 그 핀과 같은지,
  그 핀이 실제로 선언 방출을 할 수 있는지, 그리고 스캐폴더와 **복사 가능한 모든
  설치 명령**(마크다운 코드 펜스와 홈페이지 `code` 필드, 생성된 하이라이트
  포함)이 `next`를 말하는지. 산문은 면제한다 — "`typescript@7`은 7.0으로
  해석되니 쓰지 말라"는 문장을 쓸 수 있어야 하기 때문이다. 문서 목록이
  뒤처지지 않도록, 버전을 언급하는 `docs/*.md`가 목록에 없으면 그것도 실패한다.

### 결정 4: 테스트 프로젝트의 **위치**가 의존성 유무를 말한다

- **상황**: 환경 변수를 지우고 나니, 테스트가 "이 프로젝트에는 TypeScript가
  있다/없다"를 말할 수단이 사라졌다. 임시 디렉터리의 프로젝트는 위에
  `node_modules`가 없어 항상 "없다"가 된다.
- **선택과 근거**: 그것을 계약으로 삼았다. `Workspace::new`(시스템 임시
  디렉터리)는 **의존성이 없는 프로젝트**, `Workspace::in_repo`(저장소
  `target/`)는 **저장소의 의존성을 물려받는 프로젝트**다 — 모노레포의 하위
  패키지가 루트의 설치본을 쓰는 것과 같은 규칙이고, ttc가 실제로 쓰는 규칙이다.
  확장 테스트도 같은 규칙을 쓴다(`test/workspace.ts`). "TypeScript가 없을 때"를
  검사하는 케이스는 임시 디렉터리에 두면 그 자체로 조건이 성립한다.

### 결정 5: 테스트 가드의 중복 구현을 제거했다

- **상황**: `server.test.ts`와 `sidecar.test.ts`가 각자 `const COMPILER = "ttc"`
  와 자체 `compilerAvailable()`을 갖고 있었다. 공용 가드는 TASK-255에서 서버와
  같은 해석을 쓰도록 고쳤지만, 이 두 사본은 여전히 PATH만 알았다.
- **선택과 근거**: 사본을 지우고 공용 가드를 쓴다. 결과적으로 이 환경에서 26개
  스킵이 0개가 됐다 — TASK-255가 65→26으로 줄인 것을 마저 없앤 셈이다.

## 작업 내역

- 2026-08-27: 전제 재검증.
  - `npm view @typescript/native-preview versions` → 마지막 배포 2026-07-07.
  - `npm view typescript dist-tags` → `latest` 7.0.2, `next` 7.1.0-dev.20260826.1.
  - 7.0.2: API 클라이언트에 emit 계열 메서드 없음. 단 플랫폼 패키지의 실행
    파일(`lib/tsc`)로는 `--emitDeclarationOnly`가 정상 동작(확인).
  - 7.1.0-dev: `getDeclarationEmit` 존재. 환경 변수 0개로 `ttc --types`가
    `.tt-types/main.tt.d.ts` + `.map` 생성(확인).
  - 두 버전 모두에서 `ttc --server`의 hover/completion/typedCheck 정상(확인).
- 2026-08-27: `src/typescript/toolchain.rs`에서 `Source` 열거형(환경 변수·
  체크아웃·설치본)을 제거하고 설치된 패키지 해석만 남겼다. `Client`에서 `bin`
  필드가 사라지면서 `host.mjs`의 `tsserverPath` 주입도 함께 제거했다 — 클라이언트는
  자기 옆의 실행 파일을 쓴다.
- 2026-08-27: 루트 `package.json` 신규 — `typescript`를
  `7.1.0-dev.20260826.1`로 고정. `npm install`로 lockfile 생성.
- 2026-08-27: 테스트. `tests/common/mod.rs`에 `in_repo`/`in_repo_with_subdir`
  추가, `tests/native.rs` 가드를 설치본 기준으로 재작성(+ `TTC_REQUIRE_TSGO`가
  선언 방출까지 요구), `tests/corpus.rs`의 코퍼스 출처를 설치된 TypeScript의
  `lib/*.d.ts`로 교체, `tests/cli.rs`·`tests/integration.rs`의 환경 변수 조작
  제거.
- 2026-08-27: 확장. `server/src/dev.ts`와 `test/dev.test.ts` 삭제,
  `ttcSpawnEnv` 호출 제거, `findTsgo` 가드를 설치본 기준으로 축소,
  `test/workspace.ts` 추가, `server.test.ts`·`sidecar.test.ts`의 중복 가드 제거.
- 2026-08-27: npm 런처. `dev.js`의 `toolchainEnv` 제거, `bin/ttc.js`의 env 병합
  제거.
- 2026-08-27: 스크립트. `scripts/setup`은 `npm ci` + release 빌드 + 확장 설치만
  하고 인자를 받지 않는다. `scripts/doctor`는 설치된 TypeScript 버전이
  `package.json`의 핀과 같은지, 선언 방출이 가능한지 본다. `scripts/ci`는
  `TTC_TSGO_API` 대신 설치 여부를 본다.
- 2026-08-27: CI. `ci.yml`·`soak.yml`의 typescript-go 체크아웃·Go 설치·빌드
  단계를 `npm ci`로 교체하고 `TTC_TSGO_ROOT` env를 제거.
  `npm/scripts/workflow-publish-paths.test.mjs`가 그 부재를 검증한다.
- 2026-08-27: 문서. AGENTS.md, CONTRIBUTING.md, README(영·한),
  docs/getting-started(영·한), editors/vscode/README.md, npm/tt-lang/README.md,
  docs/ai/tt.md, website/src/content.json(+ 하이라이트 재생성).

## 이슈 및 해결

### 이슈 1: 임시 디렉터리의 테스트 프로젝트가 TypeScript를 못 찾음

- **증상**: 환경 변수를 지우자 확장 테스트 30개와 native 41개가 실패/스킵.
- **원인**: 테스트 프로젝트가 `/tmp`에 있어 위에 `node_modules`가 없다. 가드는
  cwd에서 위로 올라가 저장소의 설치본을 찾아 "툴체인 있음"이라고 답했고,
  실제 프로젝트에는 없었다 — 가드와 대상이 다른 곳을 보는 TASK-217의 형태.
- **해결**: 결정 4. 프로젝트의 위치를 계약으로 만들었다.

### 이슈 2: 두 테스트 파일이 공용 가드를 쓰지 않고 있었음

- **증상**: 위를 고친 뒤에도 26개가 "no ttc"로 스킵.
- **원인**: `server.test.ts`·`sidecar.test.ts`의 사본 가드가 PATH만 알았다.
- **해결**: 결정 5.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 환경 변수 0개로 전부 통과 (native 41/41, corpus 136파일)
- [x] `npm test`(editors/vscode) — 환경 변수 0개, 스킵 0
- [x] `./scripts/ci`
- [x] 소비자 E2E: npm 설치만 한 프로젝트에서 실제 language server를 stdio로
      띄워 hover·definition·completion·signature help·diagnostics 확인.
      프로젝트의 `typescript`를 치우면 전부 침묵하고 되돌리면 살아난다 —
      에디터가 프로젝트의 패키지를 쓴다는 직접 증거.
- [x] 스캐폴드 E2E: `create-tt`이 만든 프로젝트(`"typescript": "next"`)를
      손대지 않고 설치 → 7.1 나이틀리로 해석되고 `ttc --check-types`와
      `ttc --types`(사이드카 생성)까지 통과.

## 결과

TypeScript의 출처가 하나가 되었다: **프로젝트가 설치한 `typescript` 패키지.**
환경 변수도, 체크아웃도, `.tt-dev/toolchain.json`도, setup 모드 선택도 없다.
개발자는 `npm i -D @load28/tt-lang typescript@next`와 VSIX만으로 전부 동작하는
환경을 얻고(스캐폴더를 쓰면 이미 들어 있다), 저장소는 `npm ci` +
`./scripts/setup`으로 정확히 고정된 같은 툴체인을 얻는다. 대상마다 다른 그 두
스펙은 각각 한 곳에만 적히고, 다른 곳이 다른 값을 말하면 테스트가 실패한다.
