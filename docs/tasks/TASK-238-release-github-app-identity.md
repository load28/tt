# TASK-238: 전용 GitHub App 기반 릴리스 push

- **상태**: 완료
- **시작일**: 2026-08-26
- **완료일**: 2026-08-26
- **커밋**: —

## 목적

TypeScript 자동화처럼 릴리스 워크플로의 버전 커밋을 전용 GitHub App 신원으로
push한다. 해당 일반 push가 `release-*` CI를 자동으로 시작하게 한다.

## 범위

- 포함: GitHub App 설치 토큰 발급, checkout·push 인증, 워크플로 계약 테스트,
  운영 문서
- 제외: GitHub App 자체 생성과 저장소 Variable·Secret 등록, 릴리스 버전 규칙 변경

## 의사결정

### 결정 1: TypeScript와 같은 GitHub App 설치 토큰으로 push한다

- **상황**: Actions의 `GITHUB_TOKEN`으로 만든 릴리스 브랜치 push는 재귀 실행 방지
  정책 때문에 후속 CI를 시작하지 않았다.
- **검토한 대안**: 릴리스 액션이 CI를 `workflow_dispatch`로 직접 호출 / 개인 PAT 사용 /
  전용 GitHub App 설치 토큰 사용.
- **선택과 근거**: TypeScript의 실제 릴리스 워크플로도 별도 automation GitHub App
  토큰을 git 인증으로 설정한 뒤 push한다. Azure Key Vault 대신 GitHub 저장소 Secret에
  App private key를 보관하되 인증 주체와 push 기반 CI 동작은 동일하게 유지한다.

## 작업 내역

- 2026-08-26: TypeScript 봇과 TypeScript 저장소의 실제 소스를 내려받아 GitHub App
  인증, release branch push, `release-*` push CI 경로를 확인했다.
- 2026-08-26: checkout 자격 증명을 제거하고 공식 GitHub App 설치 토큰을 git
  extraheader에 설정했다. TypeScript와 같은 App 신원 push가 후속 CI를 시작한다.
- 2026-08-26: App 권한과 저장소 Variable·Secret 설정을 운영 문서에 기록하고 워크플로
  계약 테스트를 추가했다.

## 이슈 및 해결

없음.

## 검증

- [x] `node --test npm/scripts/workflow-publish-paths.test.mjs`
- [x] `./scripts/ci`

## 결과

릴리스 버전 커밋을 전용 GitHub App 설치 토큰으로 push한다. checkout에 남은
`GITHUB_TOKEN` 자격 증명은 제거했으며, 해당 App push가 `release-*` CI를 자동으로
시작한다.

변경 파일:

- `.github/workflows/release.yml`
- `npm/scripts/workflow-publish-paths.test.mjs`
- `AGENTS.md`
- `CONTRIBUTING.md`
- `docs/tasks/INDEX.md`
- `docs/tasks/TASK-238-release-github-app-identity.md`
