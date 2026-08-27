# TASK-256: TypeScript는 프로젝트가 설치한 npm 패키지 하나로

- **상태**: 완료
- **시작일**: 2026-08-27
- **완료일**: 2026-08-27
- **커밋**: `b87cac3`, `c0665a0`, `6c058f3`, `44054b6`, `e186e4f`, `39930e0`

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
  - 저장소·문서·스캐폴더가 같은 TypeScript 버전을 쓰게 하고, 그것이
    어긋나지 못하게 고정한다.
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

### 결정 3-1: 저장소·문서·스캐폴더가 모두 같은 정확한 나이틀리를 쓴다

- **상황**: 결정 3으로 저장소는 나이틀리를 고정했는데 문서는 `typescript@7`을
  안내하고 있었다. npm 범위는 프리릴리스를 매칭하지 않으므로 그 명령은 7.0.2를
  설치하고, 사용자는 사이드카를 못 쓴다. 저장소가 검증한 것과 사용자가 받는
  것이 다르다 — 이 태스크가 없애려던 바로 그 형태다.
- **검토한 대안**
  - A. 문서에 "사이드카를 쓰려면 7.1 나이틀리를 설치하라"는 각주를 단다.
    각주는 읽히지 않고, `create-tt`은 여전히 TypeScript를 설치하지 않는다.
  - B. 대상에 따라 나눈다 — 저장소는 정확한 나이틀리, 공개 안내는 `next` 태그.
    문서가 낡지 않지만, 사용자는 우리가 검증한 적 없는 나이틀리를 받게 되고
    어젯밤 올라온 업스트림 변경이 오늘 아침 빌드를 바꿀 수 있다.
  - C. 모두 같은 정확한 나이틀리를 쓴다.
- **선택과 근거**: C. 이 태스크의 주제가 "출처를 하나로"인데 버전만 둘로
  나누면 같은 종류의 틈을 다시 만든다. 정확한 버전을 명시하면 사용자는
  **컴파일러가 실제로 검증된 그 TypeScript**를 쓰고, 업스트림 나이틀리가
  올라와도 동작이 갑자기 바뀌지 않는다 — 프리릴리스에 의존하면서도 예측
  가능성을 지키는 방법이다. 문서가 낡는 문제는 7.1 정식 릴리스 때 `7`로
  **한 번에** 옮기는 것으로 끝난다. `create-tt`이 만드는 프로젝트도 같은
  버전을 받는다.
  **모두가 피해야 하는 것은 `typescript@7`이다**: 7.0 라인으로 해석되고,
  그 API 클라이언트는 선언 방출을 못 하며, 어떤 경고도 없이 사이드카만
  조용히 사라진다.
  한 문자열이 여덟 곳에 복사되므로 — 이 장애의 출발점이 정확히 그것이었다 —
  `npm/scripts/typescript-version.test.mjs`가 고정한다: 핀이 정확한 버전인지,
  설치된 것이 그 핀과 같은지, 그 핀이 실제로 선언 방출을 할 수 있는지,
  스캐폴더와 **복사 가능한 모든 설치 명령**(마크다운 코드 펜스와 홈페이지
  `code` 필드, 생성된 하이라이트 포함)이 같은 버전을 말하는지. 산문은
  면제한다 — "`typescript@7`은 7.0으로 해석되니 쓰지 말라"는 문장을 쓸 수
  있어야 하기 때문이다. 문서 목록이 뒤처지지 않도록, 버전을 언급하는
  `docs/*.md`가 목록에 없으면 그것도 실패한다.
- **확인한 전제**: 이 버전 결정은 **빌드에 영향이 없다**. ttc는 `build.rs`도
  build-dependencies도 없고 `host.mjs`를 `include_str!`로 품으므로 루트
  `node_modules`를 통째로 치워도 `cargo build --release`가 성공한다.
  `create-tt`은 순수 JS라 빌드 단계 자체가 없다. VS Code 확장만 `tsc -b`를
  쓰는데 그것은 자기 devDependency인 `typescript@^5.5.0`이고 ttc가 런타임에
  구동하는 TypeScript 7과 무관하다. 즉 여기서 정하는 버전은 순수한 **런타임**
  의존성이다.

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
  여기까지가 커밋 `b87cac3`.

- 2026-08-27: 문서가 `typescript@7`을 안내하고 있다는 지적을 받고 확인 —
  npm 범위는 프리릴리스를 매칭하지 않아 그 명령은 7.0.2를 설치한다(이슈 3).
  모든 설치 명령을 핀으로 바꾸고, `create-tt`이 그 버전을 devDependency로
  넣게 하고, `npm/scripts/typescript-pin.test.mjs`를 신규 작성했다. 작성
  직후 그 가드가 실제 드리프트 세 건을 잡았다 — README의 `typescript@7`,
  스캐폴더 누락, 재생성 안 된 홈페이지 하이라이트. 커밋 `c0665a0`.

- 2026-08-27: 대상을 나누는 안을 실제로 적용해 봤다 — 저장소는 정확한
  나이틀리, 공개 안내는 `next` 태그. 가드를 그 분리에 맞춰 다시 쓰고
  (`typescript-version.test.mjs`로 개명), `"typescript": "next"`가
  package.json에서 해석되는 것을 확인했다. 커밋 `6c058f3`.

- 2026-08-27: 그 분리를 되돌렸다(결정 3-1). 사용자가 우리가 검증한 적 없는
  나이틀리를 받게 되고 업스트림이 하룻밤 새 빌드를 바꿀 수 있다는 것이,
  문서가 낡는 비용보다 크다고 판단했다. 문서·홈페이지·스캐폴더를 모두 정확한
  핀으로 통일하고 가드를 한 값 규칙으로 되돌렸다. `create-tt` 테스트의 기대값은
  하드코딩 대신 루트 `package.json`에서 읽게 해, 버전이 실제로 한 곳에만
  적히도록 했다. 함께 확인한 전제: ttc는 루트 `node_modules`를 치워도
  `cargo build --release`가 성공하고, `create-tt`은 빌드 단계가 없으며,
  확장만 자기 devDependency인 TypeScript 5로 `tsc -b`를 한다 — 이 버전
  결정은 어떤 빌드에도 영향이 없다. 커밋 `44054b6`.

- 2026-08-27: 다시 쓴 `scripts/setup`을 처음으로 끝까지 실행해 검증했다.
  정상 종료했고 `doctor`가 `ready`로 바뀌었다. 실행 과정에서 setup의 마지막
  요약 줄이 `typescript@7`을 안내하는 드리프트를 발견해 고치고(이슈 4),
  가드가 `scripts/`가 출력하는 명령까지 보도록 확대했다. 커밋 `e186e4f`.

- 2026-08-27: 제거한 이름들을 저장소 전체에서 전수 검색해 누락 두 건을
  찾았다 — CHANGELOG에 항목이 없었고(`[Unreleased] > Changed`에 추가),
  `docs/design/compiler-core.md`의 불변식이 tsgo 세부사항의 위치로
  `native.rs`·`service.rs`만 나열하고 있었다(경계를 디렉터리로 바꾸고
  `toolchain.rs`의 역할을 명시). 커밋 `39930e0`.

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

### 이슈 3: 문서가 안내하는 버전과 저장소가 검증하는 버전이 달랐다

- **증상**: 문서의 설치 명령이 `typescript@7`이었다. 그대로 따르면 7.0.2가
  설치되고, 컴파일·타입 검사·에디터는 되는데 `ttc --types`와 사이드카만
  조용히 동작하지 않는다.
- **원인**: npm 범위 표기는 프리릴리스를 매칭하지 않으므로 `7`은 7.0 라인으로
  해석된다. 저장소는 7.1 나이틀리를 고정해 두고 사용자에게는 7.0을 안내한
  셈이었고, `create-tt`은 TypeScript를 아예 설치하지 않았다.
- **해결**: 결정 3-1. 한 버전이 여러 곳에 복사되는 것이 이 장애의 출발점이므로
  가드로 고정했다.

### 이슈 4: 스크립트가 출력하는 명령도 드리프트했다

- **증상**: `scripts/setup`의 마지막 요약이
  `a consuming project installs its own: pnpm add -D typescript@7`을 출력했다.
  가드는 문서와 스캐폴더만 보고 있어 잡지 못했다.
- **원인**: 사용자가 복사하는 명령이 문서 밖에도 있다는 것을 놓쳤다. 스크립트를
  처음 끝까지 실행해 보고서야 드러났다.
- **해결**: 문구를 핀으로 고치고, 가드가 `scripts/setup`·`doctor`·`ci`가
  출력하는 7.x 스펙까지 검사하게 했다. `scripts/ci`의 `typescript@6`은
  의도적으로 다른 일(방출물을 안정 메이저로 컴파일해 보는 오라클)이므로
  7.x 라인만 본다.

### 이슈 5: 워크플로 잡이 Node 없이 `npm ci`를 돌게 됐다

- **증상**: `soak.yml`의 corpus 잡이 `npm ci`를 실행하는데 `actions/setup-node`가
  없다. 러너 이미지에 Node가 들어 있어 "그냥 되긴" 하지만, 버전이 무엇이든
  상관없이 도는 상태다.
- **원인**: 그 잡의 typescript-go 체크아웃을 `npm ci`로 바꾸면서, 원래 Node가
  필요 없던 잡이라 setup-node가 없었다는 것을 놓쳤다. 로컬 게이트는 이
  부류를 **볼 수 없다** — 호스티드 러너에서만, 그것도 가끔만 드러난다.
- **해결**: setup-node를 넣고, 규칙을 기억에 맡기지 않도록
  `npm/scripts/workflow-publish-paths.test.mjs`가 **모든 워크플로의 모든 잡**에
  대해 "npm을 쓰면 setup-node가 있어야 한다"를 검사한다. 고친 것을 되돌리면
  그 테스트가 `soak.yml: job "corpus" runs npm without actions/setup-node`로
  실패하는 것을 확인했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 환경 변수 0개로 전부 통과 (native 41/41, corpus 136파일)
- [x] `npm test`(editors/vscode) — 환경 변수 0개, 스킵 0
- [x] `./scripts/ci`
- [x] 워크플로 계약: npm을 쓰는 모든 잡에 `actions/setup-node`가 있다
      (회귀 테스트가 실제로 잡는 것까지 확인)
- [x] 소비자 E2E: npm 설치만 한 프로젝트에서 실제 language server를 stdio로
      띄워 hover·definition·completion·signature help·diagnostics 확인.
      프로젝트의 `typescript`를 치우면 전부 침묵하고 되돌리면 살아난다 —
      에디터가 프로젝트의 패키지를 쓴다는 직접 증거.
- [x] 스캐폴드 E2E: `create-tt`이 만든 프로젝트를 손대지 않고 설치 →
      고정된 7.1 나이틀리가 들어오고 `ttc --check-types`와 `ttc --types`
      (사이드카 생성)까지 통과.
- [x] 빌드 독립성: 루트 `node_modules`를 치운 상태에서
      `cargo build --release` 성공. `create-tt`은 빌드 단계가 없고, 확장만
      자기 devDependency인 TypeScript 5로 `tsc -b`를 한다.
- [x] `./scripts/setup` 전체 실행 — 정상 종료하고 `./scripts/doctor`가
      `ready: local TT development environment is configured`로 바뀐다.
- [x] 로컬 개발 흐름 E2E: 테스트 프로젝트에 `file:` 설치 →
      `node_modules/@load28/tt-lang`이 저장소 심링크가 되고 마커가 이
      체크아웃을 가리킨다. 저장소의 `target/release/ttc`를 치우면 런처가
      그 자리를 이름 대며 실패하고 되돌리면 살아난다 — 경로를 **실행 시점에**
      해석하므로 ttc를 다시 빌드해도 재설치가 필요 없다는 증거. 같은
      프로젝트에서 실제 language server가 hover·definition·completion·
      signature help로 응답한다.

## 결과

변경 파일 (TASK-255의 커밋 이후 전부):

- **컴파일러**: `src/typescript/toolchain.rs`(해석 규칙 축소),
  `src/typescript/native.rs`, `src/typescript/host.mjs`
- **버전 고정**: `package.json`·`package-lock.json` (신규),
  `packages/create-tt/src/installer.js`,
  `npm/scripts/typescript-version.test.mjs` (신규 가드),
  `packages/create-tt/test/installer.test.mjs`
- **확장**: `server/src/dev.ts`·`test/dev.test.ts` (삭제),
  `server/src/ttc.ts`·`engine.ts`·`sidecar.ts`,
  `test/workspace.ts` (신규), 테스트 6종(`completion`·`emitmap`·`engine`·
  `server`·`sidecar`·`typedcheck`)
- **npm 런처**: `npm/tt-lang/dev.js`, `npm/tt-lang/bin/ttc.js`
- **테스트**: `tests/native.rs`, `tests/corpus.rs`, `tests/cli.rs`,
  `tests/integration.rs`, `tests/common/mod.rs`
- **스크립트·CI**: `scripts/setup`·`doctor`·`ci`,
  `.github/workflows/ci.yml`·`soak.yml`(+ corpus 잡 setup-node),
  `npm/scripts/workflow-publish-paths.test.mjs`(+ Node 설정 가드)
- **문서**: `AGENTS.md`, `CONTRIBUTING.md`, `README.md`·`README.ko.md`,
  `docs/getting-started.md`·`.ko.md`, `editors/vscode/README.md`,
  `npm/tt-lang/README.md`, `docs/ai/tt.md`,
  `docs/design/compiler-core.md`·`engine-architecture.md`, `CHANGELOG.md`,
  `website/src/content.json`(+ 하이라이트 재생성), `docs/tasks/INDEX.md`

TypeScript의 출처가 하나가 되었다: **프로젝트가 설치한 `typescript` 패키지.**
환경 변수도, 체크아웃도, `.tt-dev/toolchain.json`도, setup 모드 선택도 없다.
개발자는 `npm i -D @load28/tt-lang typescript@<핀>`과 VSIX만으로 전부 동작하는
환경을 얻고(스캐폴더를 쓰면 이미 들어 있다), 저장소는 `npm ci` +
`./scripts/setup`으로 **같은** 툴체인을 얻는다. 그 버전은 루트 `package.json`
한 곳에만 적히고, 다른 곳이 다른 값을 말하면 테스트가 실패한다. 7.1이 정식
릴리스되면 그 한 줄을 `7`로 바꾸고, 테스트가 나머지를 전부 따라오게 한다.
