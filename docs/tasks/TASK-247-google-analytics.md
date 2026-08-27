# TASK-247: 공식 홈페이지 Google Analytics

- **상태**: 완료
- **시작일**: 2026-08-27
- **완료일**: 2026-08-27
- **커밋**: `e2a12e6`

## 목적

공식 GitHub Pages 홈페이지에 Google Analytics 4 측정을 연결해 방문자와
페이지 이용 통계를 확인할 수 있게 한다.

## 범위

- 포함: 측정 ID `G-NKKYKXGD3W`의 Google 태그를 모든 홈페이지 경로에 적용,
  정적 빌드 결과 확인
- 제외: Analytics 속성·보고서 설정, 커스텀 이벤트, 광고 연동

## 의사결정

### 결정 1: 공통 루트 문서에 Google 태그를 설치

- **상황**: 현재 홈페이지는 TanStack Start가 모든 경로를 정적 프리렌더한다.
- **검토한 대안**: 각 라우트에 태그 추가 / 공통 루트 문서에 태그 추가.
- **선택과 근거**: 공통 `RootDocument`의 `<head>`에 한 번만 설치해 모든
  프리렌더 경로가 같은 측정 계약을 공유하게 한다.

## 작업 내역

- 2026-08-27: `./scripts/doctor`로 개발 환경을 확인했다.
- 2026-08-27: Google Analytics 4 측정 ID와 홈페이지 공통 루트 구조를
  확인했다.
- 2026-08-27: `RootDocument`의 `<head>`에 Google 태그 로더와 측정 초기화
  스크립트를 추가했다.
- 2026-08-27: 타입 검사와 `/tt/` 기준 정적 프리렌더를 실행하고,
  생성된 HTML에 측정 ID가 포함된 것을 확인했다.

## 이슈 및 해결

### 이슈 1: 샌드박스에서 프리렌더 미리보기 서버 시작 실패

- **증상**: `listen EPERM: operation not permitted ::1`로 최초 빌드가 중단됐다.
- **원인**: TanStack Start 프리렌더가 샌드박스에서 로컬 미리보기 포트를
  열 수 없었다.
- **해결**: 동일한 빌드를 포트 사용 권한으로 다시 실행해 37개 경로
  프리렌더를 완료했다.

## 검증

- [x] `bun run typecheck` (`website/`)
- [x] `SITE_BASE_PATH=/tt/ bun run build` (`website/`) — 37개 경로 프리렌더
- [x] 프리렌더 HTML의 Google 태그 포함 확인

## 결과

`website/src/routes/__root.tsx`의 공통 HTML `<head>`가 Google Analytics 4
측정 ID `G-NKKYKXGD3W`를 로드하고 초기화한다. 영문·한글 포함 모든
프리렌더 경로가 같은 측정 설정을 공유한다.
