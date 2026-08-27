# tt 릴리스 절차

tt은 Microsoft TypeScript와 같은 단일 개발선 모델을 사용합니다. 개발은 `main`에서
계속하고, 특정 minor 버전을 안정화할 때만 `release-X.Y`를 만듭니다. 이 문서는
개발자와 릴리스 승인자가 따라야 할 절차를 정의합니다.

## 릴리스는 어떻게 확인합니까?

Nightly와 정식 릴리스는 [GitHub Releases](https://github.com/load28/tt/releases)에서
확인합니다. 각 릴리스 후보의 CI와 게시 대기 상태는 GitHub Actions에서 확인합니다.
릴리스 계획과 포함할 작업은 해당 릴리스의 GitHub 이슈와 PR에서 관리합니다.

## 릴리스 단계는 무엇입니까?

새 릴리스 라인 `X.Y`는 Beta, RC, Stable, 그 뒤의 Patch 순서를 따릅니다.

```text
X.Y.0-beta  →  X.Y.1-rc  →  X.Y.2  →  X.Y.3  →  ...
     Beta          RC       Stable     Patch
```

`main`은 Nightly와 다음 릴리스의 개발선입니다. `release-X.Y`는 Beta에서 만들어져
Stable 뒤 Patch까지 유지됩니다. 이미 Stable `0.3.0`으로 시작한 `release-0.3`은
예외적으로 `0.3.1`부터 Patch를 이어갑니다.

## 각 단계에는 어떤 변경을 넣습니까?

### Beta

새 기능, 큰 변경, 버그 수정은 Beta 전에 `main`에 넣습니다. Beta 기간에는 RC에 포함할
변경을 `main`에서 계속 개발할 수 있습니다. RC 직전에 `main`을 릴리스 브랜치에
병합하므로, 호환성에 영향을 줄 변경은 가능한 Beta 초기에 넣어 충분히 검증합니다.

### RC

RC에는 Beta 이후의 버그 수정과 아직 끝나지 않은 에디터 연동을 넣을 수 있습니다.
RC가 나가면 `main`은 다음 minor 버전 개발선입니다. 그 뒤 현재 릴리스에 들어가는
변경은 높은 우선순위 수정으로 제한합니다.

### Stable과 Patch

RC 뒤 Stable 전에는 높은 우선순위 수정만 반영합니다. Stable 뒤 Patch에는 회귀·성능
회귀·안전한 언어 서버 수정처럼, 다음 minor까지 기다리기 어려우며 되돌려 적용할 위험이
낮은 변경만 넣습니다.

## 릴리스는 어떤 브랜치에서 만듭니까?

일반 개발은 작업 브랜치에서 시작해 PR로 `main`에 squash merge합니다. `main`은 항상
빌드 가능해야 합니다.

Beta를 시작할 때 액션이 최신 `main`에서 `release-X.Y`를 만들고 버전을
`X.Y.0-beta`로 올립니다. Beta 동안에는 `main` 전체를 릴리스 브랜치로 sync할 수
있습니다. RC 뒤에는 `main` 전체를 다시 병합하지 않습니다. 현재 릴리스에 필요한 PR의
squash merge 커밋만 `release-X.Y`에 cherry-pick합니다.

```sh
git switch release-0.4
git cherry-pick <main의-squash-merge-commit>
git push origin HEAD:release-0.4
```

충돌은 릴리스 브랜치에서 해결하고 push합니다. 그 push가 CI를 다시 시작합니다.
`release-X.Y`는 Stable 뒤에도 Patch를 위해 삭제하지 않습니다.

## 일반적인 릴리스 순서는 무엇입니까?

`0.4` 예시는 다음과 같습니다.

1. `main`에서 `release-0.4`와 `0.4.0-beta`를 만듭니다.
2. Beta 후보 CI가 성공하면 공개할 때만 게시 승인합니다.
3. Beta에 추가할 PR을 `main`에 merge하고, RC 직전에 `main`을 `release-0.4`에 sync합니다.
4. 버전을 `0.4.1-rc`로 올리고 CI 성공 후 RC 게시를 승인합니다.
5. RC 뒤에는 필요한 수정만 cherry-pick하고, 버전을 `0.4.2`로 올려 Stable 게시를 승인합니다.
6. 이후 필요한 수정만 cherry-pick하고 버전을 `0.4.3`, `0.4.4`처럼 올려 Patch를 게시합니다.

## 액션은 어떤 순서로 실행합니까?

세 액션은 각각 브랜치 생성, `main` 병합, 다음 버전 증가만 담당합니다. 액션은
`tt-release-automation` GitHub App 신원으로 버전·병합 커밋을 push합니다.

### Beta 준비

```sh
gh workflow run new-release-branch.yml --ref main -f line=0.4
```

`release-0.4`와 `0.4.0-beta`를 만듭니다.

### Beta 또는 RC 준비를 위한 sync

```sh
gh workflow run sync-release-branch.yml --ref main -f line=0.4
```

`main`을 `release-0.4`에 병합합니다. 이미 공개된 Beta와 같은 버전의 sync CI는
검증용이므로 승인하지 않습니다. 새 내용을 게시하려면 RC처럼 새 버전으로 bump합니다.

### RC·Stable·Patch 준비를 위한 bump

```sh
gh workflow run bump-release-version.yml --ref main -f line=0.4
```

현재 버전이 Beta이면 RC, RC이면 Stable, Stable이면 다음 Patch로 올립니다.

## 빌드와 게시는 어떻게 나뉩니까?

준비 액션의 push가 CI를 자동 실행합니다. CI는 모든 플랫폼 바이너리와 VSIX를 만들고
30일 동안 보관합니다. `Publish Release`는 성공한 그 CI run의 산출물만 가져오며 다시
빌드하지 않습니다.

Nightly는 예약된 `main` CI 성공 뒤 `X.Y.Z-dev.YYYYMMDD.N` 산출물을 npm `next`와
GitHub prerelease로 자동 게시합니다. `N`은 GitHub Actions 실행 번호이므로 같은 날짜의
여러 빌드도 서로 다른 불변 버전을 가집니다. 동일 실행의 재시도는 같은 버전을
재사용합니다. 소스 버전과 릴리스 브랜치는 바꾸지 않습니다.

`next`는 버전이 아니라 최신 Nightly를 가리키는 npm dist-tag입니다. 새 불변 버전을
`--tag next`로 게시할 때 포인터가 새 버전으로 이동하므로 사용자는
`@load28/tt-lang@next`로 항상 현재 Nightly를 설치할 수 있습니다.

릴리스에는 `tt-typescript-preview-<버전>-<플랫폼>.vsix`도 첨부됩니다 — 저장소가
고정한 TypeScript 나이틀리와 같은 커밋에서 빌드한 VS Code 확장으로, 마켓플레이스의
TypeScript Native Preview가 content mapper를 싣기 전까지의 에디터 짝입니다
(TASK-258). 확장 버전은 핀에서 유도되므로(`7.1.0-dev.YYYYMMDD.N` →
`0.YYYYMMDD.N`) 핀이 움직일 때만 바뀝니다. 확장 ID는 업스트림 그대로
(`TypeScriptTeam.native-preview`)입니다 — 내장 TypeScript 확장이 그 ID에만
양보하기 때문이며(TASK-259), 마켓플레이스 정식 프리뷰가 매퍼를 실으면 같은
ID의 자동 업데이트가 이 빌드를 교체하고 이 동봉은 제거합니다.

예약 실행 전에 현재 `main`을 Nightly로 게시해야 하면 CI를 수동 실행합니다.

```sh
gh workflow run ci.yml --ref main
```

수동 CI도 새 산출물을 만들고 모든 검증을 통과한 뒤 예약 CI와 같은 `next` 게시 경로로
자동 승격됩니다. 게시 workflow에 run ID나 npm tag를 직접 입력하지 않습니다.

Beta·RC·Stable·Patch는 성공한 `release-X.Y` CI 뒤 `production` Environment에서
대기합니다. 승인자는 **Approve and deploy**를 눌러야 게시됩니다. npm 태그와 CI run ID는
자동으로 선택하므로 입력하지 않습니다. 실패한 CI는 같은 브랜치에 수정 커밋을 넣어
다시 실행하고, 게시 실패는 같은 게시 job을 재실행합니다.

## 상황별로 어떻게 처리합니까?

| 상황 | 처리 |
| --- | --- |
| 다음 minor에만 넣을 변경 | PR을 `main`에 merge하고 이전 릴리스 브랜치에는 반영하지 않습니다. |
| Beta·RC 직전 다음 minor 변경 | PR을 `main`에 merge한 뒤 `sync-release-branch`를 실행합니다. |
| RC 이후 현재 릴리스의 긴급 수정 | PR을 `main`에 merge한 뒤 squash merge 커밋만 `release-X.Y`에 cherry-pick합니다. |
| 이미 게시한 버전의 Patch | 필요한 커밋을 cherry-pick하고 버전을 bump한 뒤 CI 성공 후보를 승인합니다. |
| 이전 버전에만 필요한 예외 수정 | `release-X.Y` 기반 작업 브랜치에서 PR을 만들고 그 릴리스 브랜치로 merge합니다. 이는 기본 개발선인 `main` 절차의 유지보수 예외입니다. |

## 운영 사전 조건

저장소에는 `tt-release-automation` GitHub App이 설치되어 있어야 합니다. App에는 저장소
Contents `Read and write` 권한이 필요합니다. Actions Variable `RELEASE_APP_ID`에는 App
ID를, Actions Secret `RELEASE_APP_PRIVATE_KEY`에는 App에서 내려받은 private key PEM
파일 전체를 등록합니다.

## TypeScript 원본 절차

이 문서의 “릴리스 단계”, “각 단계의 작업”, “릴리스 메커니즘”, “일반적인 순서” 구조와
브랜치 모델은 Microsoft TypeScript의 [Release Process](https://github.com/microsoft/TypeScript/wiki/TypeScript%27s-Release-Process)를 따릅니다. tt은 Azure Key Vault와 내부 봇 대신 GitHub Actions와 GitHub App을 사용한다는 환경 차이만 있습니다.
