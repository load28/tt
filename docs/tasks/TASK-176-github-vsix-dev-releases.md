# TASK-176: VS Code 개발 확장을 GitHub Release로 배포

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: 3f9a899

## 목적

Azure DevOps 조직과 Marketplace PAT 없이도 개발 중인 VS Code 확장을 배포한다.
개발 릴리스마다 VSIX를 GitHub pre-release 자산으로 올려 사용자가 직접 설치하게
한다.

## 범위

- 포함: Marketplace 게시 단계와 `VSCE_PAT` 제거, 개발 VSIX 패키징, GitHub
  pre-release 생성과 설치 안내
- 제외: VS Code Marketplace 정식·pre-release 게시, 기존 npm 개발 배포 변경

## 의사결정

### 결정 1: 개발 확장의 배포 채널은 GitHub pre-release

- **상황**: Marketplace 자동 게시에는 Azure DevOps 또는 Microsoft Entra 기반
  인증 설정이 필요하지만, 현재 개발 단계에서는 설정 비용이 반복 검증의 이익보다
  크다.
- **검토한 대안**: Marketplace PAT를 계속 설정하면 VS Code 자동 업데이트를
  쓸 수 있지만 별도 Azure 자격 증명이 필요하다. Actions artifact만 쓰면 로그인한
  저장소 사용자에게만 접근 경로가 자연스럽고 보존 기간도 제한된다. GitHub
  pre-release는 공개 다운로드 주소와 버전별 영구 기록을 제공한다.
- **선택과 근거**: 기존 GitHub 저장소 권한만으로 pre-release를 만들고 VSIX를
  첨부한다. Marketplace 자동 업데이트는 제외하고 사용자가 `Install from VSIX...`
  로 설치한다.

## 작업 내역

- 2026-08-23: TASK-175의 Marketplace 게시 흐름과 문서를 확인하고 후속 태스크를
  등록했다.
- 2026-08-23: `dev-release.yml`에서 `VSCE_PAT`와 `vsce publish`를 제거하고,
  npm 게시 성공 뒤 `softprops/action-gh-release`가 고유 개발 태그의 pre-release를
  만들며 버전된 VSIX를 첨부하도록 바꿨다. GitHub `contents: write` 권한은 해당
  작업에만 부여했다.
- 2026-08-23: `CONTRIBUTING.md`와 확장 README를 GitHub Releases 다운로드 및
  `Extensions: Install from VSIX...` 설치 절차로 갱신했다.
- 2026-08-23: 실제 `0.260823.143015` manifest로 VSIX를 패키징하고 압축 내부
  version을 확인했다. YAML 파싱, Node 테스트 10개와 전체 검증 게이트를 통과했다.

## 이슈 및 해결

### 이슈 1: 제한 환경에서 vsce registry 조회 실패

- **증상**: `npx @vscode/vsce@3.9.2 package`가 npm registry DNS를 해석하지
  못해 `ENOTFOUND`로 실패했다.
- **원인**: sandbox 네트워크 제한으로 npx가 고정 버전 패키지를 확인하지 못했다.
- **해결**: 승인된 네트워크 실행에서 동일 명령을 다시 실행해 519파일,
  1017.28KB VSIX 생성을 확인했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

개발 릴리스는 npm `dev` 게시 성공 뒤 GitHub pre-release를 생성하고 버전된 VSIX를
첨부한다. Azure DevOps 조직과 `VSCE_PAT`는 더 이상 필요하지 않다.
