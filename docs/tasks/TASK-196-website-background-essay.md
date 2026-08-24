# TASK-196: 웹사이트 배경 글 페이지

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: —

## 목적

TASK-195에서 쓴 제작 동기 글(`docs/why-tt.md`, `docs/why-tt.ko.md`)이 저장소
안에만 있어서, 웹사이트 방문자는 "왜 이 언어가 있는지"를 읽을 수 없다. 두 글을
공식 사이트(`/why`, `/ko/why`)에 싣고 README에서도 연결한다.

## 범위

- 포함: 마크다운 원문에서 사이트 콘텐츠를 생성하는 빌드 단계, 에세이 페이지
  컴포넌트와 라우트 2개, 사이트 셸 공용화, 사이드바 `배경`/`Background` 그룹,
  프리렌더·사이트맵·Pages 워크플로 입력 경로, README 링크(영/한).
- 제외: 글 내용 변경(TASK-195의 원고를 그대로 싣는다). 코드 펜스에 언어
  태그(```text)를 붙인 것만 예외 — 추출기가 요구하는 형식이다.

## 의사결정

### 결정 1: 사이트 콘텐츠를 마크다운에서 생성한다 (복붙하지 않는다)

- **상황**: 사이트는 `website/src/content.json`이라는 구조화된 콘텐츠를 쓴다.
  에세이도 같은 방식으로 JSON에 넣으면 원문이 두 벌이 된다.
- **검토한 대안**:
  - A) `docs/why-tt*.md`를 사람이 JSON으로 옮겨 적는다: 사이트 파이프라인과
    똑같이 생겨서 추가 코드가 없다. 대신 원문이 두 곳에 존재하고, 문서를 고칠
    때마다 조용히 어긋난다.
  - B) 런타임에 마크다운을 파싱한다: 마크다운 라이브러리 의존성이 생기고,
    정적 사이트에 파서를 실어 보내는 셈이다.
  - C) 빌드 단계에서 마크다운을 읽어 구조화 JSON을 생성한다: 원문은 저장소
    문서 하나뿐이고, 런타임 의존성이 없다.
- **선택과 근거**: C. `website/scripts/essay.ts`가 두 문서를 읽어
  `website/src/essay.json`을 만들고, 기존 `bun run highlight` 단계에 붙였다
  (`dev`/`build`가 이미 이 단계를 먼저 실행한다). 코드 블록은 사이트가 이미 쓰는
  shiki 하이라이터로 미리 렌더한다 — 문법 정의는 VS Code 확장의 tt 문법을
  그대로 재사용하므로 사이트의 다른 코드 블록과 동일하게 보인다.

### 결정 2: 마크다운 부분집합만 이해하고, 나머지는 빌드를 실패시킨다

- **상황**: 범용 마크다운 파서를 쓰지 않기로 했으니, 문서에 새 구문이 등장하면
  어떻게 되는지를 정해야 한다.
- **검토한 대안**: (a) 모르는 줄은 문단으로 취급해 통과 — 표나 목록이 문단으로
  뭉개져 조용히 잘못 실린다. (b) 모르는 줄은 건너뛴다 — 내용이 소리 없이
  사라진다. (c) 위치를 붙여 에러를 던지고 빌드를 실패시킨다.
- **선택과 근거**: c. 조용한 폴백은 이 저장소가 컴파일러에서 금지하는 방식이고,
  문서 파이프라인이라고 다를 이유가 없다. 지원 구문은 h1/h2, 문단, 언어 태그가
  붙은 코드 펜스, 인라인 `code`/`**강조**`/`*기울임*`/`[링크](url)`뿐이고, 그
  밖(목록·인용·표·h3 이상·태그 없는 펜스)은 `파일:행` 에러다. 실제로
  `docs/why-tt.md` 끝에 목록 한 줄을 붙여 `bun run highlight`가 exit 1로
  실패하는 것을 확인했다.
- **부수 결정**: 문서 상단의 언어 전환 줄(`[English](./why-tt.md)`)은 "링크로만
  이루어진 문단"이라는 일반 규칙으로 건너뛴다. 저장소의 이중언어 관례이고,
  사이트에는 자체 EN/한국어 토글이 있다.

### 결정 3: 레퍼런스 토픽이 아니라 별도 라우트로 싣는다

- **상황**: 사이트의 모든 페이지는 `content.json`의 토픽 스키마
  (nav/title/summary/code/works/limits)를 따르고 `ReferencePage`가 렌더한다.
  에세이에는 `code`도 `works`도 없다.
- **검토한 대안**: 에세이를 토픽 스키마에 맞춰 우겨넣기(빈 필드가 생기고
  "다음 토픽" 흐름에도 끼어든다) vs 전용 라우트 `/why`, `/ko/why`.
- **선택과 근거**: 전용 라우트. 대신 상단바·개발 중 안내·사이드바를
  `SiteShell`로 뽑아 두 페이지가 같은 셸을 공유하게 했다 — `ReferencePage`는
  186줄에서 144줄로 줄었고, 크롬은 한 벌만 남는다. 사이드바에는
  `Background`/`배경` 그룹을 더해 글 제목을 그대로 항목 이름으로 쓴다.

### 결정 4: Pages 워크플로의 트리거 경로에 문서를 추가한다

- **상황**: `.github/workflows/pages.yml`은 `website/**` 변경에만 배포한다.
  이제 사이트 내용이 `docs/why-tt*.md`에서 나오므로, 글만 고치면 배포가 돌지
  않는다.
- **선택과 근거**: `docs/why-tt.md`, `docs/why-tt.ko.md`를 트리거 경로에 추가.
  빌드 입력이 곧 트리거 입력이어야 한다.

## 작업 내역

- 2026-08-24: 사이트 구조 파악 — `website/src/content.json`(groups/topics),
  `ReferencePage.tsx`, TanStack 파일 기반 라우트, `vite.config.ts`의 프리렌더
  목록, `scripts/highlight.ts`(shiki + 사이트맵), `pages.yml`.
- 2026-08-24: `website/scripts/essay.ts` 작성 — 마크다운 부분집합 파서 +
  shiki 하이라이팅, `essayPaths` 내보내기. `scripts/highlight.ts`에
  `typescript` 문법을 추가하고 `essay.json` 생성과 사이트맵 경로를 연결.
- 2026-08-24: `docs/why-tt*.md`의 ttc 출력 펜스에 `text` 태그를 붙임(추출기가
  태그 없는 펜스를 거부한다).
- 2026-08-24: `website/src/essay.ts`(타입 접근자), `ui/SiteShell.tsx`(공용 셸),
  `ui/EssayPage.tsx`(블록 렌더러 + head 메타), `routes/why.tsx`,
  `routes/ko.why.tsx` 추가. `ReferencePage.tsx`를 셸 위로 옮김.
- 2026-08-24: `app.css`에 에세이 프로즈 스타일(본문 폭 700px, 인라인 코드,
  코드 블록, 원문 링크)과 모바일 규칙 추가. `vite.config.ts` 프리렌더에
  `/why`, `/ko/why` 추가. `pages.yml` 트리거 경로 추가.
- 2026-08-24: 검증 — `bun install --frozen-lockfile`, `bun run build`
  (프리렌더 목록에 `/why`, `/ko/why` 확인), `bun run typecheck` 통과.
  `bun scripts/preview.ts`로 띄운 정적 결과물을 Chromium(Playwright)으로
  데스크톱·모바일, 영문·한글 모두 캡처해 렌더링을 눈으로 확인.
- 2026-08-24: README 영/한에 글 링크 추가.

## 이슈 및 해결

### 이슈 1: 새 라우트가 타입 에러를 냈다

- **증상**: `bun run typecheck`에서
  `Argument of type '"/why"' is not assignable to parameter of type
  'keyof FileRoutesByPath | undefined'` (그리고 `SiteShell`의 `<Link to>`도 동일).
- **원인**: TanStack Router의 라우트 타입은 `src/routeTree.gen.ts`에서 나오고,
  이 파일은 vite 플러그인이 dev/build 때 생성한다. 새 라우트 파일만 추가한
  상태에서는 아직 생성 전이었다.
- **해결**: `bun run build`를 한 번 돌려 `routeTree.gen.ts`를 재생성하고 커밋에
  포함. 이후 타입체크 통과.

### 이슈 2: 스크린샷 확인용 Playwright가 매니페스트에 남았다

- **증상**: `bun add -d playwright`가 `website/package.json`과 `bun.lock`을
  수정했다. 사이트에 필요 없는 의존성이다.
- **원인**: 렌더링 확인 도구를 프로젝트에 설치했기 때문.
- **해결**: 확인을 마친 뒤 `git checkout website/package.json website/bun.lock`으로
  되돌렸다. 최종 diff에 두 파일은 포함되지 않는다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --no-fail-fast` — 이 태스크는 Rust 코드를 건드리지 않는다.
  `engine_cache::an_error_node_keeps_its_file_and_other_files_checkable` 실패는
  작업 환경에 `TTC_TSGO_API`가 없어서였고(TASK-195 이슈 1), 클라이언트를 지정하면
  통과한다.
- [x] `bun run typecheck` (website)
- [x] `bun run build` (website) — `/why`, `/ko/why` 프리렌더 확인
- [x] 정적 결과물 렌더링 육안 확인 (데스크톱·모바일 × 영문·한글)

## 결과

- 신규: `website/scripts/essay.ts`, `website/src/essay.ts`,
  `website/src/essay.json`(생성물), `website/src/ui/SiteShell.tsx`,
  `website/src/ui/EssayPage.tsx`, `website/src/routes/why.tsx`,
  `website/src/routes/ko.why.tsx`
- 변경: `website/scripts/highlight.ts`, `website/src/ui/ReferencePage.tsx`,
  `website/src/styles/app.css`, `website/vite.config.ts`,
  `website/src/routeTree.gen.ts`, `website/public/sitemap.xml`,
  `.github/workflows/pages.yml`, `README.md`, `README.ko.md`,
  `docs/why-tt.md`, `docs/why-tt.ko.md`(펜스 태그)

언어 표면·CLI·표준 라이브러리 동작 변경이 없으므로 `docs/ai/tt.md` 갱신은
불필요하다.
