# TASK-234: TypeScript 방식의 개발·릴리스 모델

- **상태**: 완료
- **시작일**: 2026-08-26
- **완료일**: 2026-08-26
- **커밋**: —

## 목적

기능별 Dev·Production 준비 브랜치와 별도 승인 단계를 제거한다. Microsoft TypeScript의
중앙 `main`, 장기 `release-X.Y`, 검증된 빌드 승격 모델을 tt에 적용해 개발과 릴리스를
단순화한다.

## 범위

- 포함: 개발 브랜치 계약, `main`/`release-X.Y` CI, Beta·RC·Stable 버전 규칙,
  검증된 CI 산출물의 수동 게시, 관련 스크립트·테스트·문서 정리
- 제외: 이번 작업 중 실제 npm·GitHub 배포, 자동 게시, 기존 배포 태그 삭제

## 의사결정

### 결정 1: TypeScript의 중앙 main과 릴리스 라인을 그대로 사용한다

- **상황**: 현재 구조는 작업 브랜치 외에 일회성 Dev 브랜치와 Production 브랜치까지
  만들어 같은 코드를 여러 단계에서 이동시킨다.
- **검토한 대안**: 기존 prepare/approve 구조 유지 / TypeScript의 `main`과
  `release-X.Y` 구조 적용.
- **선택과 근거**: Microsoft TypeScript 공식 릴리스 절차처럼 평상시 개발은 `main`을
  기준으로 하고, Beta 시점부터 `release-X.Y` 하나를 유지한다. RC에는 `main`을
  동기화하고, 이후 수정은 선별적으로 cherry-pick한다.

### 결정 2: 빌드와 게시는 분리하되 재빌드하지 않는다

- **상황**: 게시 승인 단계가 준비 브랜치 탐색과 빌드를 다시 소유해 흐름이 복잡하다.
- **검토한 대안**: 게시 워크플로우에서 재빌드 / CI 실행 산출물을 수동 승격.
- **선택과 근거**: TypeScript Deployment의 LKG·빌드 승격 방식처럼 CI가 정확한 SHA의
  게시 묶음을 만들고, 수동 게시 액션은 성공한 CI 실행 ID로 그 묶음만 게시한다.

### 결정 3: TypeScript의 공개 릴리스 단계와 번호를 사용한다

- **상황**: 기존 `X.Y.Z-dev.N`에서 Stable로 바로 승격하는 규칙은 TypeScript의
  Beta·RC 검증 기간과 다르다.
- **검토한 대안**: 기존 Dev/Stable 두 채널 유지 / Nightly·Beta·RC·Stable·Patch 적용.
- **선택과 근거**: Nightly는 `X.Y.Z-dev.YYYYMMDD`와 npm `next`, 릴리스 라인은
  `X.Y.0-beta` → `X.Y.1-rc` → `X.Y.2` → `X.Y.3` 순서와 npm
  `beta`·`rc`·`latest`를 사용한다.

### 결정 4: 배포는 모두 수동 승격으로 유지한다

- **상황**: TypeScript식 자동 CI와 사용자가 정한 자동 배포 금지 계약을 함께 지켜야 한다.
- **검토한 대안**: main 성공 시 Nightly 자동 게시 / 모든 채널 수동 게시.
- **선택과 근거**: CI만 자동화하고 `Publish Release`에 성공한 CI run ID와 npm 태그를
  명시해야 게시되게 했다. 이는 검증된 빌드를 승격하면서 의도하지 않은 배포를 막는다.

## 작업 내역

- 2026-08-26: Microsoft TypeScript의 공식 릴리스 절차, Deployment 문서,
  CI와 release-publish 파이프라인을 확인했다.
- 2026-08-26: `codex-task-233-release-source-workflow` 위에
  `codex-task-234-typescript-release-model` 작업 브랜치를 만들었다.
- 2026-08-26: Dev·Production 준비/승인 워크플로 네 개를 자동 CI, 릴리스 라인 진행,
  수동 게시 구조로 교체했다.
- 2026-08-26: Beta·RC·Stable·Patch 순서와 Nightly 날짜 버전 파생을 스크립트로
  구현하고 각 채널의 테스트를 추가했다.
- 2026-08-26: create-tt과 현재 설치 문서를 npm `next`·`beta`·`rc`·`latest`
  채널에 맞추고 AGENTS.md와 CONTRIBUTING.md에 새 계약을 기록했다.

## 이슈 및 해결

### 이슈 1: 기본 `codex/` 브랜치 경로를 만들 수 없음

- **증상**: `git switch -c codex/task-234-typescript-release-model`이 기존
  `refs/heads/codex` 때문에 실패했다.
- **원인**: `codex`라는 로컬 브랜치와 `codex/` 네임스페이스는 Git ref 경로를
  동시에 사용할 수 없다.
- **해결**: 저장소의 기존 규칙과 같은 `codex-task-234-typescript-release-model`을
  사용했다.

### 이슈 2: 기존 워크플로 계약 테스트가 삭제한 파일을 읽음

- **증상**: npm 테스트가 삭제된 `dev-release.yml`을 읽다가 `ENOENT`로 실패했다.
- **원인**: 테스트가 기존 네 워크플로의 파일명과 준비/승인 구조를 계약으로 고정했다.
- **해결**: 자동 CI, 장기 릴리스 브랜치, run ID 승격, 무재빌드 계약을 검증하도록
  테스트를 교체했다.

## 검증

- [x] `./scripts/ci` — agents, rust, npm, native, extension 전체 통과
- [x] GitHub Actions YAML 파싱과 릴리스 도구 테스트 23건 통과
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

일회성 Dev·Production 릴리스 브랜치와 자동 Production 게시를 제거했다. 일반 개발은
작업 PR → `main` squash merge, Nightly는 `main` CI 산출물 승격, 정식 릴리스는 하나의
`release-X.Y`에서 Beta → RC → Stable → Patch로 진행한다. 모든 게시는 수동이며 성공한
CI 산출물을 그대로 사용한다.
