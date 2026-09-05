# TASK-260: 설치 가이드를 나이틀리(next) 기준으로

- **상태**: 완료
- **시작일**: 2026-08-28
- **완료일**: 2026-08-28
- **커밋**: —

## 목적

content mapper(TASK-257~259)는 나이틀리에만 실려 있는데 공식 홈페이지와
가이드는 Stable 0.3 설치를 안내한다. 소비자 안내를 나이틀리 기준으로
갱신한다 (TASK-257 범위에서 "릴리스가 나갈 때 함께"로 미뤄 둔 항목).

## 범위

- 포함: 공식 홈페이지(website) install 토픽, README(en/ko),
  getting-started(en/ko), npm README, 확장 README, docs/ai/tt.md
  - npm 패키지는 `next` 태그로 설치 안내
  - VS Code 확장 2종(tt-language, tt-typescript-preview)을 최신 나이틀리
    릴리스 자산에서 설치 안내
  - `useTsgo` 설정(두 키) 안내 — 이유 명시: 성능을 위해 TypeScript
    7(네이티브)을 구동하고 7.1 라인 API(content mapper)를 쓰는데
    Marketplace TypeScript 확장이 아직 싣지 않아, 이 저장소의 나이틀리
    빌드를 직접 설치하고 useTsgo로 최신 API 경로를 켜야 한다
  - 7.1 정식 릴리스 후에는 확장을 공식 루트로 설치하면 되고 useTsgo
    설정도 필요 없어진다고 명시
- 제외: 설계 문서, 태스크 문서, create-tt 스캐폴더 동작 변경
  (`create-tt@0.3.0`은 Stable 채널 설치기로 그대로 안내)

## 의사결정

### 결정 1: Stable 안내를 나이틀리 안내로 교체 (병기하지 않음)

- **상황**: 매퍼 없는 Stable 0.3 경로를 병기할지.
- **선택과 근거**: 수동 설치(compiler/bundler) 안내를 `next`로 교체한다
  (사용자 결정). create-tt 절만 Stable 채널임을 기존 문구대로 유지한다.

## 작업 내역

- 2026-08-28: website install 토픽 — VS Code 확장 절을 VSIX 2종으로
  확장, "Editor TypeScript (useTsgo)" 절 신설(이유·설정·정식 릴리스 후
  전환 명시), 수동 설치 코드 `next` 태그로, works에 next 채널 항목 추가.
  `bun run highlight`로 하이라이트 재생성, `bun run build` 통과.
- 2026-08-28: README(en/ko)·getting-started(en/ko) — 확장 설치 절을 VSIX
  2종 + useTsgo + 정식 릴리스 후 안내로 확장, 수동 설치를 `next`로.
  npm README 설치 라인 `next`로. 확장 README에 useTsgo 두 키와 정식
  릴리스 후 문구 정리. docs/ai/tt.md 매퍼 항목에 에디터 전제 갱신.

## 이슈 및 해결

없음.

## 검증

- [x] `cd website && bun run build` (highlight 재생성 포함)
- [x] `node --test npm/scripts/*.test.mjs` — 34 pass
- [x] 잔여 `@load28/tt-lang@0.3.0` 안내 없음 확인 (create-tt 제외)

## 결과

설치 가이드가 실제 배포 형태와 일치한다: npm은 `next`, 에디터는 나이틀리
릴리스의 VSIX 2종 + `useTsgo`, 그리고 7.1 정식 이후의 전환 경로까지
명시된다.

변경 파일: `website/src/content.json`,
`website/src/highlighted-sections.json`(생성), `README.md`, `README.ko.md`,
`docs/getting-started.md`, `docs/getting-started.ko.md`,
`npm/tt-lang/README.md`, `editors/vscode/README.md`, `docs/ai/tt.md`,
`docs/tasks/INDEX.md`
