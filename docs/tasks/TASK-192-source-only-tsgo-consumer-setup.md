# TASK-192: 소비자 설치를 소스 빌드 tsgo로 단일화

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: 이 변경을 포함하는 커밋

## 목적

현재 공식 배포된 npm TypeScript 7은 ttc가 요구하는 sync API를 제공하지 않는다.
`create-tt`이 npm TypeScript를 설치하지 않게 하고, 현재 개발 채널의 사용자
설치 문서는 직접 빌드한 typescript-go만 안내한다.

## 범위

- 포함: `create-tt`의 `typescript` 의존성 제거, 관련 테스트와 공식
  홈페이지·GitHub 사용자 문서 갱신.
- 제외: 저장소 자체 빌드에 필요한 TypeScript 개발 의존성, CI의 독립적인 도구
  빌드 의존성, ttc/LSP의 향후 공식 npm TypeScript fallback, 로컬 setup의 npm
  모드, 과거 태스크 기록 수정.

## 의사결정

### 결정 1: 현재 배포 안내와 설치기만 소스 빌드 tsgo를 사용

- **상황**: npm TypeScript 7 설치와 typescript-go 체크아웃이 동시에 안내되어
  사용자가 둘 다 필요하다고 오해하고, 현재 npm 패키지에는 필요한 API도 없다.
- **검토한 대안**: ttc의 npm fallback까지 제거 / 설치기와 현재 사용자 안내에서만
  npm TypeScript를 제외.
- **선택과 근거**: 두 번째 방식을 선택한다. 정식 tsgo 배포 전에는 소스 빌드만
  실제 사용 경로로 안내하지만, 정식 배포 후 사용할 ttc/LSP의 npm 탐색 계약은
  유지한다.

## 작업 내역

- 2026-08-24: `create-tt`, native backend, LSP, setup, 확장 개발 주입 계층과
  사용자 문서의 npm TypeScript 7 참조를 조사했다.
- 2026-08-24: ttc/LSP와 setup의 npm fallback 제거 변경을 되돌리고, 설치기의
  생성·초기화 경로에서만 TypeScript 의존성 추가를 제거했다.
- 2026-08-24: 영문·한글 README, 시작 가이드, npm README, AI 가이드와 공식
  홈페이지를 현재 소스 빌드 tsgo 설치 절차로 갱신했다.
- 2026-08-24: `create-tt` Node 테스트, 홈페이지 타입 검사·33개 경로
  prerender와 Rust 전체 검증 게이트를 통과했다.

## 이슈 및 해결

- **증상**: 초기 범위를 ttc의 npm fallback 제거까지 넓게 해석했다.
- **원인**: 현재 실제 설치 절차와 향후 정식 배포 호환 코드를 같은 범위로 판단했다.
- **해결**: 사용자의 정정에 따라 런타임 fallback 변경을 모두 되돌리고 설치기와
  현재 공식 안내만 수정했다.

### 이슈 2: 샌드박스에서 홈페이지 prerender 포트 바인딩 실패

- **증상**: 첫 `bun run build`가 `listen EPERM: operation not permitted ::1`로
  중단됐다.
- **원인**: TanStack Start가 prerender용 로컬 preview 서버를 열지만 샌드박스가
  loopback 포트 바인딩을 제한했다.
- **해결**: 승인된 포트 권한으로 같은 빌드를 다시 실행해 33개 경로를
  prerender했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `node --test packages/create-tt/test/installer.test.mjs`
- [x] `website`: `bun run typecheck`
- [x] `website`: `bun run build` (33개 경로 prerender)

## 결과

`create-tt`은 새 프로젝트와 기존 프로젝트 초기화에서 npm TypeScript를 추가하지
않는다. 현재 공식 문서는 TT 패키지를 npm `dev` 채널에서 설치하고, ttc의
TypeScript 도구 체인은 최신 typescript-go 소스에서 직접 빌드하도록 안내한다.
ttc와 LSP의 향후 공식 npm TypeScript 탐색 기능은 유지한다.
