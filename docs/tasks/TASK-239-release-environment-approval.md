# TASK-239: 릴리스 게시 Environment 승인

- **상태**: 완료
- **시작일**: 2026-08-26
- **완료일**: 2026-08-26
- **커밋**: —

## 목적

TypeScript 게시 파이프라인처럼 성공한 릴리스 빌드와 게시 단계를 자동으로 연결하고,
개발자는 Production 승인만 수행하게 한다. CI run ID와 npm tag 수동 입력을 제거한다.

## 범위

- 포함: 성공한 CI run 자동 선택, 릴리스 tag 자동 판정, GitHub Environment 승인,
  Nightly 자동 게시 유지, 워크플로 계약 테스트와 운영 문서
- 제외: 버전 전이 규칙, 릴리스 브랜치 모델, npm 패키지 조립·게시 구현 변경

## 의사결정

### 결정 1: workflow_run과 보호된 Environment를 연결한다

- **상황**: 정식 릴리스 게시 시 사람이 CI run ID와 npm tag를 직접 입력하고 있다.
- **검토한 대안**: 기존 수동 입력 유지 / 최신 성공 run을 조회하는 수동 Action /
  CI 완료 이벤트와 GitHub Environment 승인 연결.
- **선택과 근거**: TypeScript의 upstream build resource와 Production 승인 분리를 GitHub
  Actions에서 동일한 동작으로 표현하는 세 번째 방식을 선택한다. 실행 ID는 이벤트에서,
  tag는 검증된 메타데이터에서 가져오고 게시 결정만 사람이 수행한다.

## 작업 내역

- 2026-08-26: TypeScript publish pipeline의 build resource와 approver 구조를 현재
  `release-publish.yml`의 수동 run ID 입력 방식과 대조했다.
- 2026-08-26: 성공한 CI의 run ID와 npm tag를 메타데이터에서 자동 선택하고 Nightly와
  Production Environment를 분리했다.
- 2026-08-26: 승인 뒤에도 최신 브랜치 CI인지 다시 검사해 오래된 후보와 부분 게시
  위험을 차단했다.

## 이슈 및 해결

없음.

## 검증

- [x] `node --test npm/scripts/workflow-publish-paths.test.mjs`
- [x] `./scripts/ci`

## 결과

성공한 CI 완료 이벤트가 정확한 run ID와 npm tag를 자동 선택한다. Nightly는 승인 없이
게시하고 RC·Stable·Patch는 `production` Environment에서 승인될 때까지 대기한다.
승인 뒤 최신 브랜치 CI인지 다시 확인하며 게시 단계에서는 빌드하지 않는다.

변경 파일:

- `.github/workflows/release-publish.yml`
- `npm/scripts/workflow-publish-paths.test.mjs`
- `AGENTS.md`
- `CONTRIBUTING.md`
- `docs/tasks/INDEX.md`
- `docs/tasks/TASK-239-release-environment-approval.md`
