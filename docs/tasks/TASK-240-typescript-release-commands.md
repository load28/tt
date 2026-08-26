# TASK-240: TypeScript 릴리스 명령과 Beta 단계

- **상태**: 완료
- **시작일**: 2026-08-26
- **완료일**: 2026-08-26
- **커밋**: `543b716`

## 목적

Beta를 생략한 통합 릴리스 액션을 제거하고 Microsoft TypeScript와 같이 릴리스
브랜치 생성, main 동기화, 버전 증가를 독립 명령으로 운영한다.

## 범위

- 포함: Beta·RC·Stable·Patch 버전 순서, create·sync·bump 워크플로 분리,
  Beta 게시 채널, 릴리스 문서와 계약 테스트 갱신
- 제외: Azure Pipelines와 TypeScript의 LKG 파일 복제

## 의사결정

### 결정 1: TypeScript의 세 릴리스 명령을 독립 워크플로로 둔다

- **상황**: 기존 `release.yml`은 브랜치 생성과 버전 변경을 `stage` 입력 하나로
  합쳐 TypeScript 자동화의 명령 경계와 달랐다.
- **검토한 대안**: 기존 액션에 Beta 선택지만 추가 / create·sync·bump를 독립
  워크플로로 분리
- **선택과 근거**: TypeScript 저장소의 `new-release-branch.yaml`,
  `sync-branch.yaml`, `set-version.yaml`과 같은 책임 경계를 사용한다.

### 결정 2: TypeScript와 같은 패치 번호 순서를 사용한다

- **상황**: 기존 tt 순서는 `X.Y.0-rc` → `X.Y.0` → `X.Y.1`이었다.
- **검토한 대안**: 기존 번호에 Beta 접미사만 추가 / TypeScript의 번호 순서를 복제
- **선택과 근거**: `X.Y.0-beta` → `X.Y.1-rc` → `X.Y.2` → `X.Y.3` 순서로
  진행해야 명령과 결과가 TypeScript 릴리스 모델과 일치한다.

### 결정 3: 이미 게시된 0.3 릴리스 번호는 보존한다

- **상황**: 원격 `release-0.3`과 npm/GitHub 태그에 `0.3.0-rc`, `0.3.0`이 이미
  존재하므로 TypeScript 순서인 `0.3.1-rc`, `0.3.2`로 다시 만들 수 없다.
- **검토한 대안**: 기존 태그와 브랜치 기록 재작성 / 기존 Stable 다음 Patch부터 연결
- **선택과 근거**: 게시된 SemVer는 불변으로 취급하고 `0.3.1`부터 Patch를 이어간다.
  새 릴리스 라인에는 TypeScript 순서를 온전히 적용한다.

## 작업 내역

- 2026-08-26: TypeScript의 실제 릴리스 워크플로와 봇 명령 문서를 대조했다.
- 2026-08-26: 원격 `release-0.3`과 `v0.3.0-rc`, `v0.3.0` 태그를 확인하고 기존
  릴리스 라인의 Patch 호환 규칙을 정했다.
- 2026-08-26: 통합 `release.yml`을 제거하고 Beta 브랜치 생성, `main` 동기화,
  버전 증가 워크플로를 독립 파일로 구현했다.
- 2026-08-26: Beta SemVer와 npm `beta` 태그를 산출물·게시 경로에 연결했다.

## 이슈 및 해결

없음.

## 검증

- [x] `./scripts/ci`
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] 모든 `.github/workflows/*.{yml,yaml}` YAML 구문 분석

## 결과

새 릴리스 라인은 `X.Y.0-beta`에서 생성되고 별도 sync 명령으로 `main`을 병합한 뒤,
bump 명령이 `X.Y.1-rc`, `X.Y.2`, 이후 Patch 순서로 진행한다. 각 명령의 GitHub App
push가 기존 CI와 승인 게시 흐름을 시작한다.

변경 파일:

- `.github/workflows/new-release-branch.yml`
- `.github/workflows/sync-release-branch.yml`
- `.github/workflows/bump-release-version.yml`
- `.github/workflows/release-publish.yml`
- `npm/scripts/release-bump.mjs`
- `npm/scripts/release-bump.test.mjs`
- `npm/scripts/release-artifacts.mjs`
- `npm/scripts/release-artifacts.test.mjs`
- `npm/scripts/release-version.mjs`
- `npm/scripts/release-version.test.mjs`
- `npm/scripts/workflow-publish-paths.test.mjs`
- `AGENTS.md`
- `CONTRIBUTING.md`
- `docs/tasks/INDEX.md`
- `docs/tasks/TASK-240-typescript-release-commands.md`
