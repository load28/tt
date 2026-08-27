# TASK-257: TypeScript content mapper 통합 — 사이드카 없는 `.tt` import

- **상태**: 진행 중
- **시작일**: 2026-08-27
- **완료일**: —
- **커밋**: —

## 목적

`.ts`/`.tsx`가 `.tt`/`.ttx`를 import하면 tsserver 관할에서 `TS2307`이 난다.
지금의 해법은 디스크 사이드카(`x.tt.d.ts` + `.map`)와 프로젝트별
`rootDirs`/`paths`/`tt.sidecarDir` 배선인데, 소비 프로젝트마다 설정을 요구해
DX가 나쁘다 (TASK-028, `docs/design/ts-sidecar-declarations.md`).

TypeScript 7.1이 정확히 이 문제를 위한 공개 확장점 **content mapper**를
도입했다 (typescript-go PR #4712, 7.1 iteration plan의 "Stabilize: Content
Mapper API"). ttc가 content mapper가 되면 TypeScript가 `.tt` 파일의 변환
결과를 **가상으로** 들고 검사하므로, 디스크 사이드카와 소비 측 tsconfig
배선이 필요 없어진다. Vue·Svelte·Astro가 같은 이유로 대기 중인 그 메커니즘의
의도된 사용처다.

## 범위

- 포함:
  1. `ttc --content-mapper` — content mapper 프로토콜(JSON-RPC over stdio)을
     구현하는 새 실행 모드
  2. `npm/tt-lang/package.json`의 `typescript.contentMapper` 선언 — 패키지를
     설치한 프로젝트가 tsconfig `contentMappers` 한 줄로 매퍼를 얻는다
  3. VS Code 확장의 자동 등록 — TypeScript 네이티브 프리뷰 확장의 공개
     `ExtensionAPI.registerContentMappers`를 호출해, tsconfig 없는(추론)
     프로젝트에서도 에디터 설정 0으로 동작
  4. 고정 tsgo에 대한 통합 테스트 — 실제 `tsc --runExternalCode`가 매퍼를
     구동해 `.tt` import를 검사하는 계약을 고정
  5. 문서 갱신 — 설계 문서 신설, 사이드카 설계 문서에 대체 표기,
     CONTRIBUTING의 핀 사유, `docs/ai/tt.md`, 확장 README, 저장소 README
- 제외 (후속 태스크로 등록):
  - 사이드카 기계(`--sidecar`, `--types`의 사이드카 용도, `tt.sidecar*` 설정,
    선언 방출 API 의존)의 **제거**. 매퍼가 나이틀리로 배포되어 검증되기
    전까지 기존 경로는 그대로 동작해야 한다.
  - TypeScript 7.1 정식 출시 시 핀 이동 (기존 계획,
    `npm/scripts/typescript-version.test.mjs`)
  - 공식 홈페이지(`website/`)와 getting-started의 **소비자 안내** 추가 —
    두 문서는 정식 0.3 설치를 안내하는데 0.3에는 매퍼가 없다. 매퍼가 실린
    릴리스가 나갈 때 함께 갱신한다.

## 계약 (typescript-go PR #4712 요약)

이 절은 구현이 따르는 외부 계약의 스냅샷이다. 정본은 PR #4712 본문.

### 등록

매퍼 패키지의 package.json:

```jsonc
{
  "typescript": {
    "contentMapper": {
      "exec": ["node", "bin/ttc.js", "--content-mapper"],
      // "compilerOptions": [...]  — transform이 읽는 컴파일러 옵션 (tt은 없음)
      // "dynamicConfig": true     — 외부 설정을 읽는 매퍼만 (tt은 아님)
    }
  }
}
```

소비 프로젝트의 tsconfig.json:

```jsonc
{
  "contentMappers": [
    { "package": "@load28/tt-lang", "extensions": [".tt", ".ttx"] }
  ]
}
```

CLI는 `tsc --runExternalCode`를 요구한다. VS Code는 신뢰된 워크스페이스에서만
LSP 서버에 이 플래그를 전달한다.

에디터 자동 등록 (추론 프로젝트 포함):

```ts
const ext = vscode.extensions.getExtension("TypeScriptTeam.native-preview");
const api = await ext?.activate();
api?.registerContentMappers("tt-lang.tt-language", [{
  extensions: [".tt", ".ttx"],
  inferredProject: { manifest: { name, version, exec, cwd? } },
}]);
```

### 프로토콜

stdio 위 JSON-RPC. TypeScript가 모든 요청을 보내는 단방향. 메서드 4개:

| 메서드 | 요점 |
| --- | --- |
| `initialize` | 위치 인코딩 선택(`"utf-8"` \| `"utf-16"`), `diagnosticSource` 반환 |
| `openProject` | `projectHandle` 수령, `options`/`compilerOptions` 저장. tt은 상태가 없어 빈 결과 |
| `transform` | `{fileName, content, projectHandle}` → `{text, extension, mappings?, diagnostics?, supplemental?}` |
| `closeProject` | 핸들 해제 |

`SpanMapping` = `[virtualStart, virtualLength, originalStart, originalLength,
kind, features?]`. `kind`: `Verbatim`(0, 동일 바이트 — 편집 안전) /
`Atom`(1, 대응만) / `Alias`(2, 진단에 원문 표기). `features` 생략 =
`SpanMapFeature.All`. 매핑 밖 가상 구간은 합성 코드로 취급되고, 그곳 진단은
버려지지 않고 매퍼 이름과 함께 보고된다. 텍스트 편집(rename 등)은 길이 보존
`Verbatim` 매핑으로만 되돌아온다.

실패 처리: 프로토콜 위반·크래시 시 해당 파일은 빈 TS로 취급, 한 프로젝트에서
5회 실패하면 그 매퍼는 중단된다. 디버깅: CLI `TS_CONTENT_MAPPER_DEBUG=1`,
LSP는 Trace 레벨에서 JSON-RPC와 매퍼 STDERR가 로그로 나온다.

## 의사결정

### 결정 1: 사이드카 유지·매퍼 추가 (제거는 후속)

- **상황**: 매퍼가 사이드카를 대체하므로 이번에 같이 제거할지.
- **검토한 대안**: (A) 이번에 제거 — 한 번에 끝나지만, 매퍼가 나이틀리로
  배포·검증되기 전에 유일한 경로를 없앤다. (B) 유지 + 후속 제거 — 두 경로가
  잠시 공존.
- **선택과 근거**: B. `main`은 항상 릴리스 가능해야 하고, 매퍼는 아직 어떤
  나이틀리에도 실리지 않았다. TASK-090(file: 계층 임시성)과 같은 방식으로
  제거를 후속 태스크로 명시한다.

### 결정 2: 위치 인코딩은 `"utf-8"`

- **상황**: `initialize`에서 매퍼가 인코딩을 골라야 한다.
- **검토한 대안**: `"utf-16"` — LSP 관례지만 ttc 내부는 전부 바이트 오프셋
  (AGENTS: "내부 오류는 바이트 오프셋"). 변환 계층이 하나 늘어난다.
- **선택과 근거**: `"utf-8"`. `EmitMapping`/`EmitAnchor`/진단의 바이트
  오프셋이 UTF-8 코드 유닛 오프셋과 동일하므로 변환 없이 그대로 낸다.

### 결정 3: 매핑은 `EmitMapping` → Verbatim, `EmitAnchor` → Atom(features: None)

- **상황**: ttc의 두 매핑 산출물을 SpanMapping으로 어떻게 옮길지.
- **검토한 대안**: anchor를 매핑하지 않기 — 글루 진단이 "합성 코드" 스니펫으로
  보고되어 위치를 잃는다.
- **선택과 근거**: verbatim 청크는 `Verbatim`(기본 features = All, 편집 안전).
  글루는 anchor의 `src..src_end`로 `Atom` + `SpanMapFeature.None` — 진단은
  feature 게이트가 없으므로 구성물 키워드 위치로 매핑되고(기존 anchor 계약과
  동일), 내비게이션·rename은 글루로 절대 들어가지 않는다(기존 "glue는
  진단 전용" 계약 보존). anchor는 innermost-first로 정렬되어 있고 가상 구간이
  겹치면 안 되므로, 안쪽 anchor를 우선해 겹침을 잘라낸다.

### 결정 4: transform의 진단은 tt 수준 체크만

- **상황**: `transform`이 `diagnostics`(원본 파스 오류)를 반환할 수 있다.
- **검토한 대안**: 진단 없이 텍스트만 — tt 문법 오류가 TS 파스 오류로 둔갑해
  에러 계층 계약(§2)을 깬다.
- **선택과 근거**: `compile`급 tt 수준 체크의 진단을 `MapperDiagnostic`으로
  반환한다(`diagnosticSource: "tt"`). 타입 오류는 TypeScript의 것 — 매퍼는
  타입을 모른다. 기존 에러 계층이 프로토콜 위에서 그대로 성립한다.
  낮춤 자체가 막힌 파일(projection 차단 진단)은 빈 모듈 + tt 진단을 낸다 —
  TypeScript가 실패한 매퍼 파일에 대입하는 것과 같은 형태이고, 원인은 원본
  위치의 tt 진단이 그대로 전달한다.

### 결정 5: exec은 launcher 경유 (`node bin/ttc.js --content-mapper`)

- **상황**: `exec`이 매퍼 프로세스를 지정한다.
- **검토한 대안**: 플랫폼 바이너리 직접 지정 — package.json은 정적이라
  플랫폼 분기가 불가능하다.
- **선택과 근거**: 기존 launcher(`bin/ttc.js`)가 플랫폼 패키지와 로컬 개발
  설치(dev.js)를 이미 해석한다. 인자를 그대로 전달하므로 추가 작업 없음.
  에디터 추론 프로젝트 매니페스트도 같은 launcher를 exec으로 쓴다
  (TASK-255의 설치 패키지 해석을 재사용).

## 구현 계획

### 1. Rust: `src/content_mapper.rs` — 프로토콜 서버

- `--content-mapper` 플래그를 `src/main.rs`에 추가, 새 모듈로 진입.
- JSON-RPC 2.0 over stdio. 프레이밍은 `Content-Length` 헤더(vscode-jsonrpc
  기준)로 구현하되, **스모크 프로브로 먼저 실측 확인**한다 (아래 5).
- `initialize` → `{ positionEncoding: "utf-8", diagnosticSource: "tt" }`.
- `openProject`/`closeProject` → 핸들 집합만 유지, 빈 결과. tt 매퍼는
  프로젝트 상태·옵션·컴파일러 옵션을 쓰지 않는다 (정적 identity —
  incremental에서 프로세스 스폰 없이 up-to-date 판정).
- `transform`:
  - `fileName` 확장자로 `SourceKind` 결정: `.tt` → 출력 `".ts"`,
    `.ttx` → `".tsx"`.
  - `ttc::emit_mapped_with_kind` → `text`.
  - `mappings`: `EmitMapping` → `[out, len, src, len, Verbatim]`;
    `EmitAnchor` → `[out, end-out, src, src_end-src, Atom, None]`
    (안쪽 우선, 가상 구간 겹침 제거).
  - `diagnostics`: tt 수준 체크 결과를 바이트 오프셋으로.
  - `supplemental`/`diagnosticDirectives`: 사용하지 않음.
- 알 수 없는 메서드는 JSON-RPC 오류로 응답하고 세션은 유지 (server.rs와
  같은 태도). stdout은 프로토콜 전용, 로그는 stderr.

### 2. npm: 패키지 선언

- `npm/tt-lang/package.json`에 `typescript.contentMapper` 필드 추가
  (`exec: ["node", "bin/ttc.js", "--content-mapper"]`).
- `npm/scripts/*` 패키지 검증 테스트가 있으면 필드 추가를 반영.

### 3. VS Code 확장: 자동 등록

- `editors/vscode/client/src/extension.ts`에서 활성화 시:
  - `vscode.extensions.getExtension("TypeScriptTeam.native-preview")` 조회
    (부재 시 조용히 건너뜀 — 사이드카 경로가 그대로 커버).
  - `activate()` 후 `registerContentMappers("tt-lang.tt-language", [...])`.
  - `inferredProject.manifest`는 워크스페이스에 설치된 `@load28/tt-lang`을
    해석해(서버 `install.ts`의 해석 로직 재사용 또는 클라이언트 측 동등
    구현) `exec`을 구성. 미설치 시 확장 번들 launcher로 폴백하지 않고
    등록을 생략한다 — 확장이 임의 컴파일러를 주입하지 않는다는 TASK-255
    계약 유지.
  - 반환된 Disposable을 `context.subscriptions`에 등록.
- 네이티브 프리뷰 확장의 실제 ID는 구현 시점에 확인해 고정한다
  (스펙 예제는 `TypeScriptTeam.native-preview`).

### 4. 테스트

- Rust 단위: 프레이밍 파서, transform 매핑 산출(패스스루 파일 = 전체
  Verbatim 1개, variant/match 파일 = 글루 Atom 검증), `.ttx` 확장자.
- 통합 (`tests/`): 고정 tsgo 실물 구동 —
  `.tt`을 import하는 `.ts` + tsconfig `contentMappers` 픽스처에
  `tsc --runExternalCode`를 돌려 (a) 클린 검사 통과, (b) 의도된 타입 오류가
  원본 `.tt` 위치로 보고됨을 고정. TypeScript는 TASK-256 계약대로 저장소
  `node_modules`의 것 하나.
- 확장: 등록 로직 단위 테스트 (API 부재/존재 mock).

### 5. 선행 스모크 프로브 (구현 전)

Node 스크립트 ~30줄로 트리비얼 매퍼를 만들어 고정 tsgo에 물려
`TS_CONTENT_MAPPER_DEBUG=1 tsc --runExternalCode`로 실측한다:
- JSON-RPC 프레이밍(Content-Length 여부), 필드 이름, 요청 순서 확인
- 이 결과를 본 태스크 문서 "이슈 및 해결"에 기록하고 1의 구현에 반영

### 6. 문서

- `docs/design/content-mapper.md` 신설 — 이 통합의 설계 (계약 스냅샷,
  매핑 규칙, 에러 계층과의 관계).
- `docs/design/ts-sidecar-declarations.md` — 머리에 "content mapper로 대체
  진행 중, TASK-257" 표기.
- `CONTRIBUTING.md` — 나이틀리 핀 사유에 content mapper API 추가.
- `docs/ai/tt.md` — `.ts`에서 `.tt` import 항목: 매퍼 경로 우선, 사이드카는
  레거시로.
- `editors/vscode/README.md` — "`.ts`·`.tsx`에서 가져다 쓰기" 절을 매퍼
  기준으로 재작성 (사이드카 절은 유지하되 레거시 표기).
- `README.md`/`README.ko.md` — 소비 프로젝트 안내 갱신.
- `AGENTS.md` — 아키텍처 경계에 content mapper 진입점 한 줄.

## 작업 내역

- 2026-08-27: 태스크 생성. 계약 조사 완료 — 고정 나이틀리(7.1.0-dev.20260826.1)
  바이너리에서 `contentmapper` 기계와 `--runExternalCode` 확인, 정식 7.0.2에는
  부재(문자열 0건) 확인. PR #4712 본문(스펙)과 네이티브 프리뷰 확장의
  `ExtensionAPI.registerContentMappers` 소스 확인.
- 2026-08-27: 스모크 프로브 (계획 5) — Node 트리비얼 매퍼를 고정 tsgo에 물려
  실측: 프레이밍은 `Content-Length: N\r\n\r\n{...}`(LSP base protocol),
  JSON-RPC 2.0, id는 `"api1"` 같은 문자열. `initialize` params는
  `{ positionEncodings: ["utf-8", "utf-16"] }`, `openProject`는
  `{ configFileName, projectHandle, compilerOptions }`, `transform`은
  `{ fileName, content, projectHandle }`. 매핑된 진단은 원본 위치로,
  매핑 밖 진단은 "no corresponding location" 주석과 함께 보고됨을 확인.
- 2026-08-27: `src/content_mapper.rs` 구현 + `--content-mapper` 배선
  (`src/main.rs`). 단위 테스트 4개(프레이밍 구간 분할, 패스스루 = 전체
  Verbatim 1개, 글루 = Atom+features None+비겹침, 코드 번호 안정성).
- 2026-08-27: 고정 tsgo E2E — 클린 통과(exit 0), 소비자 타입 오류
  `main.ts(2,7): TS2322`, 1-hop exhaustiveness `partial.tt(4,10): error tt27`,
  글루 타입 오류 `wrong.tt(4,3): TS2322`(match 위치), `@tt/std` 물질화,
  `.ttx`→`.tsx` 확인. `tests/content_mapper.rs` 통합 스위트 6케이스로 고정.
- 2026-08-27: `npm/tt-lang/package.json`에 `typescript.contentMapper` 선언
  (launcher 경유 exec). 실제 패키지 복사본 + launcher 경유 E2E 재확인.
- 2026-08-27: VS Code 확장 자동 등록 —
  `editors/vscode/client/src/contentMapper.ts` 신설, activate에서
  fire-and-forget 호출. 워크스페이스의 `@load28/tt-lang`을 `binaryPath()`로
  해석해 추론 프로젝트 매니페스트 제공(미설치 시 확장자만 등록 — TASK-255의
  "확장은 임의 컴파일러를 주입하지 않는다" 계약 유지). `npx tsc -b` 통과.
- 2026-08-27: 문서 — `docs/design/content-mapper.md` 신설,
  `ts-sidecar-declarations.md` 대체 표기, CONTRIBUTING 핀 사유,
  `docs/ai/tt.md` Workflow 절 재작성(매퍼 우선·사이드카 레거시),
  `editors/vscode/README.md` 절 재구성, README(en/ko)·npm README·
  getting-started(en/ko) 핀 사유 갱신, AGENTS 아키텍처 경계 한 줄.

## 이슈 및 해결

### 이슈 1: 가상 확장자 거부 — "unsupported virtual extension 'ts'"

- **증상**: 첫 E2E에서 `TS100025: The content mapper '@load28/tt-lang'
  failed to transform this file. … unsupported virtual extension 'ts'`.
- **원인**: `SourceKind::output_extension()`은 점 없는 `"ts"`를 반환하는데
  프로토콜의 `MappedOutput.extension`은 점을 포함한 `".ts"`를 요구한다.
- **해결**: 매퍼 응답에서 `format!(".{}", …)`로 점을 붙였다. 통합 테스트가
  두 확장자 모두 고정한다.

### 이슈 2: tt 진단이 있는 파일은 TypeScript 의미 검사를 건너뛴다

- **증상**: 한 파일에 exhaustiveness 오류와 타입 오류를 함께 넣자 tt
  진단만 보고되고 타입 오류가 나오지 않았다.
- **원인**: 매퍼의 `diagnostics`는 "원본 파스 오류"로 취급되고, TypeScript는
  구문 오류가 있는 파일의 의미 검사를 건너뛴다 — TypeScript 자신의 파일과
  같은 규칙이다.
- **해결**: 결함이 아니라 계약으로 기록 (`docs/design/content-mapper.md`).
  tt 오류를 고치면 타입 오류가 그 다음에 보인다 — `ttc --check` →
  `--check-types` 순서와 같은 경험이다.

### 이슈 3: 에디터 등록 인터페이스가 스펙 본문 예제와 다르다

- **증상**: PR #4712 본문 예제는 `inferredProjectContribution` 필드를 쓴다.
- **원인**: 그것은 LSP 직렬화 형태이고, 확장 API의 입력 타입
  (`ContentMapperContribution`)은 `inferredProject`다. 본문 예제가 코드보다
  오래됐다.
- **해결**: 확장 소스(`_extension/src/contentMapperContributions.ts`)를
  정본으로 삼아 `inferredProject`로 구현하고 설계 문서에 기록했다.

### 이슈 4: 로컬 `./scripts/ci`의 extension 스테이지 실패 — 기존 문제

- **증상**: `install.test.js` 3건이 `/private/var/...` vs `/var/...` 경로
  불일치로 실패.
- **원인**: macOS의 `/var` → `/private/var` 심링크. `require.resolve`는
  정규화된 경로를 답하는데 테스트 기대값은 `os.tmpdir()` 그대로다. 이
  브랜치의 diff에는 server 소스 변경이 없고, main의 코드로도 같은 3건이
  같은 방식으로 실패한다 — 이 태스크와 무관한 이 머신 고유의 기존
  문제다(원격 CI는 ubuntu라 통과). 별도 태스크 감이다.
- **해결**: 이 태스크에서는 손대지 않는다.

### 이슈 5: 첫 실행에서 hand-written `.ts`의 `@tt/std` import가 TS2307

- **증상**: `node_modules/@tt`가 아직 없는 프로젝트에서, `.ts` 파일이
  `@tt/std`를 직접 import하면 첫 `tsc --runExternalCode`가 TS2307을 낸다.
  재실행은 클린 통과.
- **원인**: 매퍼 프로세스는 첫 `.tt` 조회 때 게으르게 스폰되고, 물질화는
  `openProject`/`transform`에서 일어난다. 그런데 hand-written `.ts`의
  bare `@tt/std` 해석은 그보다 먼저 실행될 수 있고, 실패가 기록된 뒤에
  파일이 생긴다. `.tt`/`.ttx`만 `@tt/std`를 쓰는 프로젝트(rlx-tour 형태)는
  가상 트리 해석이 transform 뒤라 겪지 않는다.
- **해결**: 이 태스크에서는 한계로 기록한다. 매퍼 계층에서는 스폰 시점을
  당길 수 없다. 근본 해법은 `@tt/std`를 실제 npm 패키지로 배포해 물질화
  자체를 없애는 것(TASK-090의 방향)이며 후속으로 미룬다. 우회는 재실행
  또는 아무 ttc 실행 한 번.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (신규 `tests/content_mapper.rs` 6케이스 포함)
- [x] `./scripts/ci` — rust·npm·agents·native 통과. extension 스테이지는 이 머신 고유의 기존 실패(이슈 4)
- [x] 통합: 고정 tsgo `tsc --runExternalCode` 픽스처 통과
- [x] 실제 패키지 선언 + launcher 경유 E2E (수동)

## 결과

`.ts`/`.tsx`가 `.tt`/`.ttx`를 import하는 경로가 TypeScript 7.1 content
mapper로 동작한다: 소비 프로젝트는 tsconfig `contentMappers` 한 줄
(+ CLI `--runExternalCode`), 에디터는 확장이 자동 등록하므로 설정 0.
디스크 사이드카·`rootDirs`/`paths`는 이 경로에서 더 이상 필요 없다.
사이드카는 레거시 경로로 유지된다(결정 1). 진단은 원본 위치로 — 소비자
타입 오류, `.tt` 내부 타입 오류(글루는 구성물 위치), tt 수준 규칙
(`tt<N>` 코드) 모두 통합 테스트 6케이스로 고정했다.

변경 파일:

- 신규: `src/content_mapper.rs`, `tests/content_mapper.rs`,
  `editors/vscode/client/src/contentMapper.ts`,
  `docs/design/content-mapper.md`, 이 문서
- 수정: `src/main.rs`(`--content-mapper` 배선·usage),
  `npm/tt-lang/package.json`(`typescript.contentMapper`),
  `editors/vscode/client/src/extension.ts`(자동 등록 호출),
  `docs/design/ts-sidecar-declarations.md`(대체 표기), `AGENTS.md`,
  `CONTRIBUTING.md`, `docs/ai/tt.md`, `editors/vscode/README.md`,
  `README.md`, `README.ko.md`, `npm/tt-lang/README.md`,
  `docs/getting-started.md`, `docs/getting-started.ko.md`,
  `docs/tasks/INDEX.md`

후속 태스크(범위의 제외 항목): 사이드카 기계 제거, 7.1 정식 핀 이동과
함께 진행. 릴리스 시 website·getting-started 소비자 안내 추가.
