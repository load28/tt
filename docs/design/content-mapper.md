# TypeScript content mapper 통합

TASK-257에서 작성했다. `.ts`/`.tsx`가 `.tt`/`.ttx`를 import하는 경로를
TypeScript 7.1의 content mapper로 옮긴 설계다. 사이드카 설계
([`ts-sidecar-declarations.md`](./ts-sidecar-declarations.md), TASK-028)를
대체하며, 사이드카는 content mapper가 없는 TypeScript를 위한 레거시
경로로 유지된다.

## 문제와 해법

사이드카의 문제는 파일이 아니라 배선이었다: 소비 프로젝트마다
`ttc --types` 옵트인, `rootDirs`/`paths`, `tt.sidecarDir` 설정이 따라붙었다.
TypeScript 7.1은 정확히 이 부류의 도구(Vue·Svelte·Astro의 템플릿 검사기)를
위한 공개 확장점을 도입했다: **content mapper** — 외부 프로세스가 낯선
파일을 TypeScript 텍스트로 변환해 주면, TypeScript가 그것을 **가상으로**
들고 모듈 해석·타입 검사·언어 서비스까지 수행한다. 디스크에 아무것도
쓰지 않는다.

정본 계약은 typescript-go PR #4712이고, 7.1 iteration plan이 이 API의
안정화를 명시한다. 와이어 사실(프레이밍, 파라미터 형태)은 고정 나이틀리
`typescript@7.1.0-dev.20260826.1`에 대한 실측으로 확인했다(TASK-257).

## 구성 요소

| 조각 | 역할 |
| --- | --- |
| `src/content_mapper.rs` (`ttc --content-mapper`) | 프로토콜 프로세스. JSON-RPC 2.0 over stdio, `Content-Length` 프레이밍. `initialize`/`openProject`/`transform`/`closeProject` |
| `npm/tt-lang/package.json`의 `typescript.contentMapper` | TypeScript가 매퍼 프로세스를 스폰하는 방법: `["node", "bin/ttc.js", "--content-mapper"]` — launcher가 플랫폼 바이너리와 로컬 개발 설치를 해석한다 |
| 소비 tsconfig의 `contentMappers` | `{ "package": "@openload28/tt-lang", "extensions": [".tt", ".ttx"] }` 한 줄. CLI는 `tsc --runExternalCode` |
| VS Code 확장 `client/src/contentMapper.ts` | TypeScript 확장의 `registerContentMappers`에 `.tt`/`.ttx`를 자동 등록. 워크스페이스의 `@openload28/tt-lang`이 있으면 그 `binaryPath()`로 추론 프로젝트 매니페스트까지 제공 |

## transform의 계약

- **텍스트**: [`ttc::compile_report`]의 방출 — CLI가 컴파일하는 것과 같은
  코드. `.tt` → `".ts"`, `.ttx` → `".tsx"`. 상대 `.tt`/`.ttx` 지정자는
  재작성하지 않는다(`ImportRewrite::Off`): `contentMappers.extensions`가
  모듈 해석에 그 확장자를 가르치므로, 가상 트리 안에서 서로를 그대로
  찾는다.
- **위치 인코딩 `"utf-8"`**: ttc의 스팬은 전부 바이트 오프셋이고 UTF-8
  코드 유닛이 곧 그 바이트라 변환 계층이 없다.
- **매핑**: `EmitMapping`(원본에서 바이트 단위로 복사된 청크) →
  `Verbatim` 스팬(기본 features = All — 편집 안전). `EmitAnchor`(컴파일러가
  쓴 글루) → 그 구성물의 원본 구간(`src..src_end`)으로 가는 `Atom` 스팬 +
  `SpanMapFeature.None`. 진단은 feature 게이트가 없으므로 글루의 타입
  오류가 구성물 위치로 보고되고(기존 anchor 계약과 동일), 내비게이션·rename은
  글루로 절대 들어가지 않는다. 가상 스팬은 겹칠 수 없으므로 anchor는
  innermost-first로 빈 구간만 채운다.
- **진단**: tt 수준 규칙(`ttc --check`와 같은 [`ttc::compile_report`]의
  진단)을 `diagnosticSource: "tt"`로 반환한다. 코드는
  `CODE_NUMBERS`(append-only 표)의 안정 번호 — `match-not-exhaustive`는
  `tt27`로 렌더된다. 타입 오류는 TypeScript의 것 — 에러 계층 계약(§2)이
  프로토콜 위에서 그대로 성립한다. 한 파일에 tt 진단이 있으면 TypeScript는
  그 파일을 구문 오류가 있는 파일처럼 다루어 의미 검사를 건너뛴다(실측).
- **exhaustiveness**: CLI와 같은 1-hop 수집 — 직접 상대 import를 디스크에서
  읽어 [`ttc::exported_variants_with_kind`]로 모은다. 캐시하지 않는다:
  프로세스가 `--watch` 아래에서 편집보다 오래 살기 때문이다.
- **`@tt/std`**: 방출이 bare 지정자를 import하므로, `openProject`의
  tsconfig 루트(또는 파일에서 올라가 찾은 패키지 루트)에
  `node_modules/@tt/{std,runtime}`를 물질화한다 — typed engine이 언어
  서비스에 하는 것과 같은 규칙, 이미 있으면 절대 덮어쓰지 않는다.
- **정적 identity**: `dynamicConfig` 없음, `compilerOptions` 요구 없음 —
  tt의 변환은 프로젝트 설정과 무관하다. incremental/`--build`의 up-to-date
  판정이 매퍼 프로세스를 스폰하지 않고 끝난다.

## 실측으로 확인한 것 (2026-08-27, 7.1.0-dev.20260826.1)

- 프레이밍: `Content-Length: N\r\n\r\n{...}` — LSP base protocol. id는
  `"api1"` 같은 문자열.
- `initialize` params는 `{ positionEncodings: ["utf-8", "utf-16"] }`.
- 가상 확장자는 점을 포함한다(`".ts"` — `"ts"`는
  "unsupported virtual extension").
- 매핑된 진단은 원본 위치로("`src/partial.tt(4,10): error tt27`"), 매핑
  밖 진단은 매퍼 이름과 "no corresponding location" 주석과 함께 보고된다.
- 에디터 등록 인터페이스는 PR 본문 예제가 아니라 확장 소스가 정본:
  `ContentMapperContribution.inferredProject`(본문 예제의
  `inferredProjectContribution`은 직렬화 형태), `manifest.cwd`는
  `vscode.Uri`.

## 사이드카와의 관계

| | 사이드카 (TASK-028) | content mapper (TASK-257) |
| --- | --- | --- |
| 디스크 | `.tt.d.ts` + `.map` 생성 | 없음 |
| 소비 설정 | `--types` 옵트인 + `rootDirs`/`paths` (+ 에디터 설정) | tsconfig 한 줄, 에디터 0 |
| 요구 TypeScript | 아무 tsserver | 7.1+ (`--runExternalCode`) |
| 타입 검사 주체 | tsserver가 선언만 봄 | TypeScript가 변환 전체를 검사 |

제거는 후속 태스크로 미룬다: 매퍼가 나이틀리로 배포·검증되기 전까지
사이드카는 유일하게 증명된 경로다. TypeScript 7.1 정식 출시(핀 이동)와
같은 시기가 자연스럽다.
