# TASK-188: 설치 문서 역할 분리와 typescript-go 소스 연동 안내

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: 이 변경을 포함하는 커밋

## 목적

npm의 TypeScript 7 패키지에 ttc가 사용하는 sync API가 아직 포함되지 않은 개발
단계의 제약을 정확히 안내한다. 웹사이트는 일반 사용자의 공식 배포 설치 흐름만
설명하고, 저장소 개발 환경은 GitHub README에 분리한다.

## 범위

- 포함: 영문·한글 웹 설치 페이지, 영문·한글 README와 설치 가이드의
  typescript-go 및 로컬 개발 안내 정리.
- 제외: 컴파일러·설치 CLI·배포 워크플로 동작 변경, 패키지 버전 변경.

## 의사결정

### 결정 1: typescript-go만 소스에서 빌드하고 TT 구성 요소는 공식 배포본을 사용

- **상황**: 현재 npm의 TypeScript 7에는 ttc가 쓰는 sync API가 없지만, 웹의
  로컬 Verdaccio 절차는 일반 설치자가 TT 패키지까지 직접 게시하게 만든다.
- **검토한 대안**: 모든 의존성을 로컬 빌드하는 절차 유지 / TypeScript 7 npm
  패키지만 사용 / typescript-go만 최신 소스에서 빌드하고 TT 패키지는 npm
  공식 배포본 사용.
- **선택과 근거**: 세 번째 방식을 선택한다. `src/typescript/native.rs`와
  `scripts/setup`에서 요구하는 실행 파일과 sync API를 같은 체크아웃에서 만들 수
  있고, 사용자가 요청한 공식 배포와 개발용 예외의 경계에도 맞는다.

### 결정 2: 제품 설치와 저장소 개발 절차를 문서 위치로 분리

- **상황**: 웹 설치 페이지에 컴파일러 기여자용 로컬 레지스트리 명령이 섞여 있다.
- **검토한 대안**: 웹에 두 절차를 모두 유지 / 웹에는 제품 설치만 두고 저장소
  개발 절차는 GitHub README와 설치 가이드에 둔다.
- **선택과 근거**: 두 번째 방식을 선택한다. 일반 설치자는 npm 배포본과 현재
  필요한 typescript-go 예외만 보고, 저장소 기여자는 GitHub에서 재현 가능한
  로컬 빌드·검증·레지스트리 절차를 찾을 수 있다.

### 결정 3: 현재 공식 설치 채널을 `dev`로 명시

- **상황**: CI가 개발 버전을 npm `dev` dist-tag로 배포하고 production
  `latest`는 갱신하지 않는다.
- **검토한 대안**: 미래 정식 배포를 가정해 `latest` 유지 / 현재 실제 배포
  채널인 `dev` 명시.
- **선택과 근거**: `create-tt@dev`, `tt-lang@dev`, `unplugin-tt@dev`를 명시한다.
  `.github/workflows/dev-release.yml`과 TASK-175의 채널 계약을 확인했고,
  `create-tt@dev`가 생성 의존성도 `dev`로 맞추는 구현을 확인했다. 로컬
  Verdaccio는 게시 스크립트가 별도로 `latest`를 쓰므로 그 재현 명령은 유지한다.

## 작업 내역

- 2026-08-23: 웹 설치 콘텐츠, README, 설치 가이드, 실제 toolchain 해석 순서와
  `scripts/setup`을 조사했다. Microsoft typescript-go의 현재 공개 저장소와 API
  개발 상태도 확인했다.
- 2026-08-23: 웹의 로컬 Verdaccio 절차를 제거하고 typescript-go 빌드,
  `TTC_TSGO_ROOT`, npm `dev` 채널 기반 자동·수동 설치 절차로 교체했다.
- 2026-08-23: 영문·한글 README와 설치 가이드, 배포 패키지 README, AI 가이드에
  공식 `dev` 설치와 저장소 개발 환경의 경계를 반영했다.
- 2026-08-23: `bun run highlight`, 웹 타입 검사와 production prerender 빌드,
  Rust 검증 게이트 전체를 실행했다.

## 이슈 및 해결

### 이슈 1: 샌드박스에서 prerender 서버 포트 바인딩 실패

- **증상**: `bun run build`가 `listen EPERM: operation not permitted ::1`로
  prerender 단계에서 실패했다.
- **원인**: TanStack Start의 prerender 미리보기 서버가 로컬 포트를 열어야 하지만
  샌드박스가 포트 바인딩을 차단했다.
- **해결**: 동일 명령을 허용된 환경에서 재실행해 33개 영문·한글 경로의
  prerender 완료를 확인했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `website`: `bun run typecheck`
- [x] `website`: `bun run build` (33개 경로 prerender)

## 결과

공식 홈페이지에는 typescript-go 최신 소스 빌드와 npm `dev` 채널의 사용자 설치
절차만 남겼다. GitHub의 영문·한글 README와 설치 가이드에는 같은 공식 설치법과
별도로 `scripts/setup`, 로컬 레지스트리, 검증 게이트를 포함한 저장소 개발
환경을 기록했다. 컴파일러와 배포 동작은 변경하지 않았다.
