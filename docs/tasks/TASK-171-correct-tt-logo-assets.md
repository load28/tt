# TASK-171: TT 로고 자산 교정

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: —

## 목적

rl에서 tt로 개명한 뒤에도 남은 기존 `rl` 모노그램을 제거한다. 웹사이트와
VS Code에서 파일명뿐 아니라 실제로 그려지는 글자까지 `tt`/`ttx`로 통일한다.

## 범위

- 포함: 웹사이트 파비콘, VS Code `.tt`·`.ttx` 파일 아이콘의 SVG 도형.
- 제외: 웹사이트 본문 레이아웃, 확장 기능, 언어 동작 변경.

## 의사결정

### 결정 1: 기존 색상·이중 획을 유지하고 글자 도형만 교체

- **상황**: 아이콘은 차콜 외곽선과 라임 중심 획으로 라이트·다크 테마의 대비를
  함께 확보한다. 문제는 배색이 아니라 경로가 여전히 `rl`을 그린다는 점이다.
- **검토한 대안**: 새 배색과 조형을 전면 설계하면 브랜딩 범위가 커진다. SVG
  `<text>`는 간단하지만 에디터 아이콘에서 시스템 글꼴에 따라 모양이 달라진다.
- **선택과 근거**: 기존 팔레트와 획 구조를 보존하고 두 개의 소문자 `t`, TTX는
  두 개의 `t`와 `x`를 고정 경로로 그린다. 파비콘은 기존 구현 방식대로 텍스트만
  `tt`로 교체한다.

## 작업 내역

- 2026-08-23: 웹사이트 파비콘의 인라인 SVG가 `rl` 텍스트를 포함하고, 이름이
  바뀐 VS Code 아이콘도 기존 `rl`/`rlx` 경로를 유지한 사실을 확인했다.
- 2026-08-23: TASK-171을 등록하고 세 SVG 표현을 `tt`/`ttx`로 교체했다.
- 2026-08-23: 두 파일 아이콘을 256px와 실제 표시 크기인 32px PNG로 렌더링해
  `tt`·`ttx` 판독성과 투명 배경을 확인했다. SVG XML 파싱도 통과했다.
- 2026-08-23: 웹사이트 TypeScript 검사와 33개 경로 프리렌더링, VS Code 컴파일과
  아이콘 계약을 포함한 grammar 테스트 10개를 통과했다.
- 2026-08-23: Rust 필수 게이트 세 개를 통과하고 수정한 아이콘이 포함된 TT VS Code
  확장을 제거 후 재설치했다.

## 이슈 및 해결

### 이슈 1: 저장소 개명 전 Cargo 캐시가 옛 실행 파일 경로를 포함함

- **증상**: 기본 `cargo test -q`에서 CLI 테스트 32개가 `failed to run ttc: No
  such file or directory`로 실패했다.
- **원인**: `env!("CARGO_BIN_EXE_ttc")`를 담은 기존 테스트 바이너리가 저장소를
  `/rl`에서 `/tt`로 옮기기 전에 생성되어 삭제된 옛 절대 경로를 실행했다. 현재
  `target/debug/ttc`와 `target/release/ttc` 바이너리는 모두 존재했다.
- **해결**: `CARGO_TARGET_DIR=/private/tmp/tt-task171-target`로 새 target에서 전체
  테스트를 다시 빌드했다. 모든 테스트 묶음이 통과했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `CARGO_TARGET_DIR=/private/tmp/tt-task171-target cargo test -q`
- [x] 웹사이트 `bun run typecheck`, `bun run build` (33 pages prerendered)
- [x] VS Code `npm run compile`, grammar 테스트 (10 passed)
- [x] SVG XML 파싱과 256px·32px 렌더링 확인

## 결과

- `website/src/routes/__root.tsx`: 인라인 파비콘 텍스트를 `tt`로 교체했다.
- `editors/vscode/icons/tt-file.svg`: 두 소문자 `t`를 그리는 경로로 교체했다.
- `editors/vscode/icons/ttx-file.svg`: 두 소문자 `t`와 `x`를 그리는 경로로 교체했다.
- `.tt-dev/tt-language.vsix`를 다시 패키징하고 설치된 TT 확장을 갱신했다.
