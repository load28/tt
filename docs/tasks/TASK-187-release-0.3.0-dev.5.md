# TASK-187: release 0.3.0-dev.5

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: `460958e`

## 목적

모든 npm 패키지를 `@load28` 스코프로 통일한 `0.3.0-dev.5` 개발 릴리스를
자동 배포하고 npm 패키지 여덟 개와 GitHub VSIX pre-release를 확인한다.

## 범위

- 포함: Cargo 기준 버전을 `0.3.0-dev.5`로 변경, CI와 자동 Dev Release,
  scoped npm `dev` 패키지와 VSIX 결과 확인
- 제외: production 배포, npm `latest` dist-tag 변경

## 의사결정

### 결정 1: 부분 게시된 dev.4 대신 dev.5 사용

- **상황**: dev.4의 기존 언스코프 플랫폼 패키지와 `tt-lang`은 게시됐지만
  `unplugin-tt`에서 작업이 실패했다. dev.5부터는 모든 package spec이 새
  `@load28` 스코프이므로 전체 패키지를 최초 게시해야 한다.
- **검토한 대안**: dev.4를 재실행하면 이미 게시된 언스코프 버전과 충돌하고
  버전 증가 기반 자동 트리거도 검증하지 못한다. 새 기준 버전은 scoped 패키지
  전체에 동일한 새 immutable 버전을 부여한다.
- **선택과 근거**: Cargo 기준 버전을 `0.3.0-dev.5`로 올려 CI 성공 뒤 자동
  Dev Release가 scoped 패키지 전체를 게시하게 한다.

## 작업 내역

- 2026-08-23: TASK-187을 등록하고 Cargo 기준 버전을 `0.3.0-dev.5`로 변경했다.
- 2026-08-23: Node 패키지·설치기 테스트 21건과 세 검증 게이트를 통과했다.
- 2026-08-23: 커밋 `460958e`을 `main`에 push한 뒤 CI run
  `32643676331`과 자동 Dev Release run `32643872446`의 성공을 확인했다.
- 2026-08-23: npm 사용자 패키지 세 개와 플랫폼 패키지 다섯 개가 모두
  `dev` 태그로 게시됐음을 Actions 로그와 npm 공개 상태로 확인했다.
- 2026-08-23: GitHub pre-release
  `dev-v0.3.0-dev.5.20260823.135629.10.1`에
  `tt-language-0.260823.135629.vsix`가 첨부됐음을 확인했다.

## 이슈 및 해결

### npm의 linux-arm64 익명 조회 응답 불일치

- **증상**: 게시 직후 `npm view @load28/tt-lang-linux-arm64`가 E404를 반환했다.
- **원인 조사**: Actions 로그에서 동일 패키지와 버전에 대한 npm의 성공 응답을
  확인했고, `npm access get status`는 패키지 상태를 `public`으로 반환했다.
- **해결**: 게시 성공 로그와 공개 접근 상태를 릴리스 검증 근거로 남겼다.
  익명 `npm view`의 E404 원인은 확인되지 않았으므로 응답 불일치로 기록했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

`@load28` 스코프의 npm 패키지 여덟 개를 개발 채널로 게시했다. 자동 Dev
Release가 성공했고 VSIX를 포함한 GitHub pre-release도 생성됐다.
