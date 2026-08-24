# TASK-197: tt 제작 동기 글 한글판 문장 개정

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: 이 변경을 포함하는 커밋

## 목적

TASK-195에서 작성한 한글 동기 글(`docs/why-tt.ko.md`)의 문장을 사용자가 직접
다듬은 원고로 교체한다. 기존 한글판은 영문판을 구어체로 옮긴 초벌 번역에
가까웠고, 문어체·경어체가 섞이고 용어(타입스크립트/TypeScript)가 흔들렸다.

## 범위

- 포함: `docs/why-tt.ko.md` 전문 교체, 제목 변경에 따른 `README.ko.md` 링크
  문구 수정, 웹사이트 생성 산출물(`website/src/essay.json`) 재생성.
- 제외: 영문판(`docs/why-tt.md`) 문장 수정 — 이번 개정은 한글판 문체 문제이며,
  영문판은 그대로 두어 절 구성만 1:1로 유지한다. 컴파일러 코드 변경 없음.

## 의사결정

### 결정 1: 한글판을 번역문이 아니라 독립 원고로 취급한다

- **상황**: 사용자가 제공한 원고는 영문판의 직역이 아니라 같은 논지를 한국어
  글로 다시 쓴 것이다. 이를 그대로 반영할지, 영문판과 문장 단위로 맞출지
  정해야 했다.
- **검토한 대안**:
  - 대안 A: 영문판에 맞춰 직역 톤을 유지 — 두 문서의 대조는 쉬우나, 사용자가
    고친 이유(한국어로 읽히는 글) 자체가 사라진다.
  - 대안 B: 제공된 원고를 그대로 채택하고 절 구성만 영문판과 1:1로 유지 —
    문체는 한국어 기준으로 자연스럽고, 웹사이트의 언어 토글이 같은 절 순서를
    보이므로 구조적 대응은 유지된다.
- **선택과 근거**: 대안 B. 절 제목 8개가 영문판 8개와 순서대로 대응하고,
  생성된 `essay.json`의 블록 수가 en/ko 모두 37개로 동일함을 확인했다
  (`python3`로 `blocks` 길이 비교).

### 결정 2: 마크다운 표기는 기존 문서 관례를 그대로 적용한다

- **상황**: 제공된 원고는 평문이라 인라인 코드·강조·코드 펜스 표기가 없었다.
  웹사이트 빌더(`website/scripts/essay.ts`)는 지원하는 마크다운 부분집합만
  파싱하고, 벗어나면 빌드를 실패시킨다.
- **검토한 대안**: 평문 그대로 두기(코드가 본문에 섞여 하이라이팅 불가) vs
  기존 한글판과 동일한 표기 규칙 적용.
- **선택과 근거**: 후자. 식별자·키워드는 백틱, 핵심 원칙 한 줄은 `**강조**`,
  예제는 ```ts / ```tt / ```text 펜스로 두어 기존 문서와 같은 규칙을 지켰다.
  `bun run highlight`가 오류 없이 통과하는 것으로 부분집합 준수를 확인했다.

### 결정 3: 제목 변경을 참조 지점까지 전파한다

- **상황**: 제목이 "tt를 만들게 된 이유" → "tt를 만든 이유"로 바뀌었다.
  제목과 요약은 문서 첫 줄과 첫 단락에서 생성되므로 웹사이트는 자동 반영되지만,
  `README.ko.md`의 링크 문구는 수동이다.
- **선택과 근거**: `README.ko.md:96`의 링크 텍스트를 새 제목으로 맞췄다.
  문서와 참조가 어긋나면 버그로 취급한다는 저장소 규칙(CLAUDE.md)을 따른 것.

## 작업 내역

- 2026-08-24: `docs/why-tt.ko.md`를 사용자가 제공한 원고로 전면 교체.
  절 구성(8개 `##`), `[English](./why-tt.md)` 내비게이션 줄, 코드 펜스 4개는
  기존 문서와 동일하게 유지.
- 2026-08-24: `README.ko.md:96`의 링크 문구를 새 제목으로 수정.
- 2026-08-24: `website/`에서 `bun install` 후 `bun run highlight`로
  `website/src/essay.json` 재생성. `bun run typecheck`, `bun run build`
  (프리렌더 포함) 통과 확인.
- 2026-08-24: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test` 실행. 마지막 항목은 아래 이슈 1 참조.

## 이슈 및 해결

### 이슈 1: `bun run highlight`가 `shiki/wasm` 모듈을 찾지 못했다

- **증상**: `error: Cannot find module 'shiki/wasm' from
  '/root/.bun/install/cache/shiki@4.4.3@@@1/dist/bundle-full.mjs'`.
- **원인**: 컨테이너에 `website/node_modules`가 없어 bun이 전역 캐시의 부분
  설치본을 물었다.
- **해결**: `website`에서 `bun install`(164 packages) 후 재실행하니 정상
  생성됐다. 저장소 결함이 아니다.

### 이슈 2: `cargo test`에서 `engine_cache` 테스트 1건이 실패했다

- **증상**: `an_error_node_keeps_its_file_and_other_files_checkable`가 진단
  2건을 기대하는데 1건(`stray-pipe`)만 받아 실패.
- **원인**: TASK-195에서 기록된 것과 동일한 환경 문제. 이 컨테이너에 TypeScript 7
  API 클라이언트가 없어 엔진이 백엔드 답을 받지 못한다. 이번 변경은 문서·생성
  산출물뿐이라 Rust 경로와 무관하다.
- **해결**: `npm install --no-save typescript@7` 후
  `TTC_TSGO_API=$PWD/node_modules/typescript/dist/api/sync/api.js cargo test`로
  재실행해 전체 통과(실패 0건)를 확인했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (`TTC_TSGO_API` 지정, 전체 통과)
- [x] `bun run typecheck`, `bun run build` (website)

## 결과

- `docs/why-tt.ko.md`: 한글 동기 글 전문 교체 (제목 포함).
- `README.ko.md`: 링크 문구를 새 제목에 맞춤.
- `website/src/essay.json`: 문서에서 재생성 (ko 블록 37개, en과 동일).

언어 표면·CLI·표준 라이브러리 동작 변경이 없으므로 `docs/ai/tt.md` 갱신은
필요하지 않다.
