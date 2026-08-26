# 릴리스 운영 가이드

tt은 Microsoft TypeScript의 릴리스 모델을 따릅니다. 개발은 항상 `main`에서 하고, 출시할 릴리스 라인만 `release-X.Y`에서 안정화합니다. Nightly는 자동 게시하며, Beta·RC·Stable·Patch는 사람이 게시 승인합니다.

## 공통 원칙

- 기능·버그 수정은 작업 브랜치에서 개발하고 PR로 `main`에 squash merge합니다.
- `main`은 다음 릴리스의 개발선이며, 항상 빌드 가능해야 합니다.
- `release-X.Y`는 Beta부터 Patch까지 유지하는 해당 minor 버전의 안정화·유지보수선입니다.
- 릴리스 액션이 `tt-release-automation` GitHub App 신원으로 버전 커밋을 push합니다. 사람이 버전 커밋이나 CI dispatch를 따로 만들지 않습니다.

## 자동과 수동의 경계

릴리스 준비 액션은 브랜치를 만들거나 버전을 올린 뒤 push합니다. 그 push가 CI를 자동 실행합니다. CI가 성공하면 `Publish Release`가 성공한 해당 CI 산출물만 받아 게시 후보를 만들고 `production` Environment에서 대기합니다. 승인자는 GitHub Actions의 대기 중인 `Publish Release`에서 **Approve and deploy**만 누릅니다.

사람이 직접 하는 일은 “준비 액션 실행”과 “게시 승인” 두 가지입니다. CI run ID, npm 태그, 산출물 선택은 직접 입력하지 않습니다. 실패한 CI는 같은 브랜치에 수정 커밋을 추가하여 다시 돌립니다. 게시가 실패한 경우에는 같은 게시 job을 재실행합니다.

## Nightly: `main`의 최신 상태를 자동 게시

개발자가 별도로 하는 일은 없습니다. 매일 예약된 `main` CI가 현재 소스를 `X.Y.Z-dev.YYYYMMDD`로 산출물에만 스탬프하고, 성공하면 npm `next`와 GitHub prerelease에 자동 게시합니다. 소스의 버전 파일과 릴리스 브랜치는 바꾸지 않습니다.

## 새 minor 릴리스: Beta에서 Stable까지

### 1. Beta를 시작합니다

새 버전 `0.4`를 시작할 시점의 `main`에서 다음을 실행합니다.

```sh
gh workflow run new-release-branch.yml --ref main -f line=0.4
```

액션이 `release-0.4`를 만들고 `0.4.0-beta` 버전 커밋을 push합니다. CI가 성공하면 게시 후보가 대기하므로, Beta를 공개할 때만 승인합니다.

### 2. Beta에 추가 변경을 넣습니다

추가 변경은 평소처럼 작업 브랜치 → PR → `main` 순서로 반영합니다. RC에 포함할 준비가 되면 다음을 실행합니다.

```sh
gh workflow run sync-release-branch.yml --ref main -f line=0.4
```

이 액션은 `main` 전체를 `release-0.4`에 병합하고 CI를 자동 시작합니다. 이 시점까지는 새 기능·변경을 Beta에 포함할 수 있습니다. 필요하면 같은 sync를 반복합니다. 이미 게시한 Beta와 같은 버전의 sync CI는 검증용이므로 승인하지 않고, 다음 RC 버전을 bump한 뒤 게시합니다.

### 3. RC를 만듭니다

RC 직전에 마지막으로 `main`을 sync한 뒤 다음을 실행합니다.

```sh
gh workflow run bump-release-version.yml --ref main -f line=0.4
```

액션은 `0.4.0-beta`를 `0.4.1-rc`로 올리고 CI를 시작합니다. CI 성공 뒤 RC를 게시할 때 승인합니다. RC가 나간 뒤 `main`은 다음 minor 버전 개발선으로 봅니다.

### 4. Stable을 만듭니다

RC 뒤에는 새 기능을 sync하지 않습니다. 현재 릴리스에 꼭 필요한 높은 우선순위 수정만 아래 “기존 릴리스 수정” 절차로 `release-0.4`에 반영합니다. 안정화가 끝나면 다음을 실행합니다.

```sh
gh workflow run bump-release-version.yml --ref main -f line=0.4
```

액션은 `0.4.1-rc`를 `0.4.2`로 올립니다. CI 성공 뒤 승인하면 npm `latest`와 GitHub 정식 릴리스로 게시됩니다.

## 기존 릴리스 수정: Stable 전과 Patch

RC 이후 Stable 전의 긴급 수정과, 이미 Stable로 게시된 버전의 수정은 같은 방식입니다. 수정 PR은 우선 `main`에 squash merge합니다. 그 PR의 squash merge 커밋만 해당 `release-X.Y`에 cherry-pick합니다. `main` 전체를 병합하지 않습니다.

```sh
git switch release-0.4
git cherry-pick <main의-squash-merge-commit>
git push origin HEAD:release-0.4
```

충돌이 나면 릴리스 브랜치에서 해결하고 CI가 성공했는지 확인합니다. 이 커밋은 다음 버전의 준비물이므로 아직 게시 승인하지 않습니다. Stable 뒤 Patch를 게시할 때는 다음을 실행합니다.

```sh
gh workflow run bump-release-version.yml --ref main -f line=0.4
```

액션은 `0.4.2` 다음을 `0.4.3`, 그다음은 `0.4.4`로 올립니다. 각 버전은 CI 성공 후 승인해야 게시됩니다. `release-X.Y`는 Patch를 위해 삭제하지 않습니다.

## 상황별 처리

| 상황 | 개발자가 할 일 |
| --- | --- |
| 다음 minor에만 포함 | PR을 `main`에 merge하고 이전 `release-X.Y`에는 반영하지 않습니다. |
| Beta·RC 직전의 다음 minor 변경 | PR을 `main`에 merge한 뒤 `sync-release-branch`를 실행합니다. |
| RC 이후 현재 릴리스의 긴급 수정 | PR을 `main`에 merge한 뒤 그 squash merge 커밋만 `release-X.Y`에 cherry-pick합니다. |
| 이미 게시된 이전 버전의 Patch | 필요한 커밋만 `release-X.Y`에 cherry-pick하고 버전을 bump한 뒤 승인합니다. |
| 이전 버전에만 필요한 예외적 수정 | `release-X.Y` 기반 작업 브랜치에서 PR을 만들고 해당 릴리스 브랜치로 merge합니다. 이는 TypeScript의 기본 개발선인 `main` 절차가 아닌 유지보수 예외입니다. |

## 버전 순서와 기존 0.3 라인

새 릴리스 라인 `X.Y`의 버전은 `X.Y.0-beta` → `X.Y.1-rc` → `X.Y.2` → `X.Y.3` … 순서입니다. 이미 `0.3.0` Stable을 게시한 `release-0.3`은 이 모델 도입 전부터 존재하므로 `0.3.1`부터 Patch만 이어갑니다.

## 운영 사전 조건

저장소에는 `tt-release-automation` GitHub App이 설치되어 있어야 합니다. App에는 저장소 Contents `Read and write` 권한이 필요합니다. Actions Variable `RELEASE_APP_ID`에는 App ID를, Actions Secret `RELEASE_APP_PRIVATE_KEY`에는 App에서 내려받은 private key PEM 파일 전체를 등록합니다. `production` Environment의 승인자는 게시 직전에 확인합니다.

## TypeScript 원본 절차

이 흐름의 브랜치 모델과 Beta·RC·Stable·Patch 순서는 Microsoft TypeScript의 [Release Process](https://github.com/microsoft/TypeScript/wiki/TypeScript%27s-Release-Process)를 따릅니다. tt은 Azure Key Vault와 내부 봇 대신 GitHub Actions와 GitHub App을 사용한다는 환경 차이만 있습니다.
