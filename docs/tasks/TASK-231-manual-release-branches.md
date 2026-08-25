# TASK-231: 수동 릴리스 브랜치 기반 Dev·Production 배포

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-26
- **커밋**: —

## 목적

Dev와 Production 준비를 수동 workflow로 시작하고 별도 릴리스 브랜치에서 버전
커밋과 전체 플랫폼 빌드를 검증한다. Dev 게시는 수동 승인으로 태그만 남기고,
Production은 검증된 Dev 태그에서 만든 릴리스 PR이 `main`에 머지될 때 자동 게시한다.

## 범위

- 포함: 선택적 `X.Y.Z` 입력, 자동 버전 계산, 미완료 릴리스 브랜치 재사용,
  Dev 기반 Production 승격, 전체 플랫폼 사전 빌드, 준비·승인 분리,
  준비 SHA·artifact 검증, Production PR, 멱등 npm 게시와 릴리스 스크립트 테스트
- 제외: Production 릴리스 PR 머지 외의 자동 배포, npm 인증 방식 변경

## 의사결정

### 결정 1: Production PR 머지만 자동 게시를 허용

- **상황**: 임의 push나 태그가 게시를 시작하면 안 되지만 Production 승인과
  main 반영은 하나의 행위여야 한다.
- **검토한 대안**: 모든 게시를 수동 dispatch / push·tag 자동 게시 /
  Production 릴리스 PR 머지만 자동 게시.
- **선택과 근거**: 준비와 Dev 게시는 수동으로 유지한다. 같은 저장소의
  `release/vX.Y.Z` PR이 main에 실제로 머지된 이벤트만 Production을 게시한다.

### 결정 2: Dev 태그를 Production PR로 승격

- **상황**: Dev는 Production 후보 검증이므로 main 이력을 먼저 바꿀 필요가 없지만,
  최종 Production 버전과 코드는 main에 반영돼야 한다.
- **검토한 대안**: Dev·Production 모두 main에 머지 / 모두 태그만 보존 /
  Dev는 태그, Production은 PR 머지.
- **선택과 근거**: Dev 승인은 태그만 만들고 Production은 그 태그에서 stable
  브랜치와 main 대상 PR을 만든다. 사람이 PR을 머지하는 행위가 Production
  승인이며, 그 머지 이벤트만 자동 게시를 시작한다.

### 결정 3: Production은 성공한 같은 core의 Dev만 승격

- **상황**: stable에 Dev에서 검증하지 않은 소스가 섞이는 경로를 막아야 한다.
- **검토한 대안**: 현재 main 직접 stable 배포 / 성공 Dev 태그의 SHA 승격.
- **선택과 근거**: Production 대상 `X.Y.Z`와 같은 core의 최신 성공
  `X.Y.Z-dev.N` 태그를 선택한다. 현재 main이 그 Dev 커밋의 조상이어야 하며,
  main이 별도로 진행됐다면 최신 main에서 새 Dev를 만든 뒤 다시 승격한다.

## 작업 내역

- 2026-08-25: 기존 workflow, 버전 스탬프와 채널 계산 계약을 확인했다.
- 2026-08-25: 두 릴리스 게이트가 고정된 typescript-go를 직접 빌드하고
  `TTC_TSGO_ROOT`로 참조하도록 기존 native CI 계약과 맞췄다.
- 2026-08-26: 릴리스 준비와 사람의 승인·게시를 별도 수동 workflow로
  분리하고, 릴리스 커밋을 main에 반영하지 않기로 결정했다.
- 2026-08-26: 준비 성공 상태에 정확한 릴리스 SHA와 workflow run을 기록하고,
  승인 workflow가 그 실행의 artifact만 게시하도록 구현했다.
- 2026-08-26: Dev는 태그까지만 남기고, Production 준비가 main 대상 PR을
  만들며 그 PR의 실제 머지만 Production 게시를 자동 시작하도록 변경했다.

## 이슈 및 해결

- Node 24의 `node --test` 요약 기호가 `#`에서 `ℹ`로 달라 로컬 extension
  게이트가 `skipped 0`을 오판했다. 두 출력 형식을 모두 허용하되 0건 검사 계약은
  그대로 유지했다.

## 검증

- [x] 릴리스 계획·버전 계산·스탬프 단위 테스트 9개
- [x] workflow 정적 계약 테스트 6개와 YAML 구문 검사
- [x] `./scripts/ci` (`agents`, `rust`, `npm`, `native`, `extension`)

## 결과

Dev 준비와 게시는 수동이며 성공하면 Dev 태그만 남긴다. Production 준비는 현재
main의 계보에 있는 성공 Dev 태그에서 stable 브랜치를 만들고 검사·빌드한 뒤
main 대상 PR을 생성한다. 같은 저장소의 준비된 Production PR이 실제 머지된
경우에만 검증된 artifact를 자동 게시하고 Production 태그를 만든다.
