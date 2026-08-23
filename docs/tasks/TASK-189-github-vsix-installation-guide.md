# TASK-189: GitHub VSIX 확장 설치 안내

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: 이 변경을 포함하는 커밋

## 목적

현재 개발 단계의 TT VS Code 확장은 Marketplace가 아니라 GitHub pre-release로
배포된다. 공식 홈페이지와 GitHub 설치 문서에서 VSIX 다운로드·설치 경로를
사용자 설치 흐름의 일부로 명시한다.

## 범위

- 포함: 웹 설치 페이지의 GitHub Releases 링크와 VSIX 설치 절차, 영문·한글
  README와 설치 가이드의 같은 안내, 홈페이지 빠른 설치 명령의 `dev` 채널 정합성.
- 제외: 확장 빌드·배포 워크플로 변경, Marketplace 배포 추가.

## 의사결정

### 결정 1: 고정 자산 URL 대신 GitHub Releases 페이지를 연결

- **상황**: 개발 릴리스마다 태그와 `tt-language-<버전>.vsix` 파일명이 달라진다.
- **검토한 대안**: 특정 릴리스 자산 URL 고정 / Releases 목록에서 최신 pre-release
  VSIX를 선택하도록 안내.
- **선택과 근거**: Releases 목록을 연결한다. `.github/workflows/dev-release.yml`이
  각 개발 버전마다 pre-release와 버전된 VSIX를 생성하므로 고정 링크의 노후화를
  피하면서 실제 배포 계약을 그대로 설명할 수 있다.

## 작업 내역

- 2026-08-23: 개발 릴리스 워크플로와 확장 README에서 VSIX 파일명, GitHub
  pre-release, `Extensions: Install from VSIX...` 설치 계약을 확인했다.
- 2026-08-23: 웹 설치 페이지에 GitHub Releases 링크, VSIX 파일 선택 기준,
  명령 팔레트와 `code --install-extension` 설치법을 영문·한글로 추가했다.
- 2026-08-23: 영문·한글 README와 설치 가이드, npm 컴파일러 패키지 README에
  같은 GitHub VSIX 설치 경로를 기록했다.
- 2026-08-23: 홈페이지 빠른 설치 버튼에 남아 있던 `create-tt@latest`를 현재
  공식 개발 채널인 `create-tt@dev`로 교정했다.

## 이슈 및 해결

없음.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `website`: `bun run typecheck`
- [x] `website`: `bun run build` (33개 경로 prerender)

## 결과

공식 홈페이지와 GitHub 설치 문서가 최신 GitHub pre-release의
`tt-language-<버전>.vsix`를 내려받아 VS Code 명령 팔레트 또는 CLI로 설치하도록
안내한다. 웹의 다운로드 문구는 GitHub Releases로 직접 연결된다.
