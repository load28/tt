# TASK-236: VS Code와 tsgo CI 통합

- **상태**: 완료
- **시작일**: 2026-08-26
- **완료일**: 2026-08-26
- **커밋**: `5fcb1dc`

## 목적

npm `typescript@7`을 사용하는 VS Code CI의 무한 대기를 제거한다. pinned typescript-go를
직접 빌드하는 하나의 잡에서 네이티브 백엔드와 VS Code 확장을 함께 검증한다.

## 범위

- 포함: GitHub CI 잡 통합, release metadata 의존성, 워크플로 계약 테스트
- 제외: 테스트 코드와 typescript-go 핀 변경

## 의사결정

### 결정 1: pinned tsgo 잡을 단일 통합 게이트로 사용한다

- **상황**: `vscode extension` 잡은 npm TypeScript 7을 설치한 뒤 테스트가 끝나지 않지만,
  pinned typescript-go를 빌드하는 native 잡의 동일 확장 테스트는 성공한다.
- **검토한 대안**: npm 경로에 timeout 추가 / 두 잡 모두 checkout 기반으로 유지 /
  native 잡 하나로 통합.
- **선택과 근거**: native 잡은 이미 네이티브 테스트와 확장 전체 테스트를 연속 실행한다.
  중복된 VS Code 잡을 제거하면 같은 compiler/API 쌍을 한 번 빌드해 모든 계약을 검증한다.

## 작업 내역

- 2026-08-26: 이전 PR CI 두 건에서 npm TypeScript 7 기반 VS Code 잡만 30분 이상
  `Test`에 머물고 pinned typescript-go 잡은 약 4분 안에 통과함을 확인했다.
- 2026-08-26: 독립 VS Code 잡을 제거하고 native 잡의 이름과 의존성을 통합 게이트로
  정리했다. 통합 잡에는 30분 실행 상한을 설정했다.
- 2026-08-26: CI에서 npm `typescript@7` 설치가 없고 release metadata가 통합 잡만
  기다리는지 계약 테스트로 고정했다.

## 이슈 및 해결

없음.

## 검증

- [x] `./scripts/ci` — agents, rust, npm, native, extension 전체 통과
- [x] GitHub Actions YAML 파싱과 워크플로 계약 테스트 4건 통과

## 결과

GitHub CI는 pinned typescript-go의 실행 파일과 API client를 한 번 빌드하는
`tsgo type checking + vscode extension` 잡에서 네이티브·코퍼스·확장 테스트를 모두
실행한다. npm TypeScript 7 기반 중복 잡은 제거했다.
