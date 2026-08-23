# TASK-193: release 0.3.0-dev.6

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: 이 변경을 포함하는 커밋

## 목적

TASK-192의 소스 빌드 tsgo 소비자 설치 단일화를 npm `dev`와 GitHub VSIX
개발 채널로 배포한다.

## 범위

- 포함: `Cargo.toml` 기준 버전을 `0.3.0-dev.6`으로 올리고 lockfile과 태스크
  기록을 갱신한 뒤 `main`에 푸시.
- 제외: TASK-192 외 기능 변경, production 배포와 npm `latest` 갱신.

## 의사결정

### 결정 1: 기준 버전만 dev.6으로 올림

- **상황**: TASK-192 변경을 자동 개발 배포가 감지할 수 있는 새 버전이 필요하다.
- **검토한 대안**: npm 패키지 버전도 저장소에서 직접 수정 / `Cargo.toml`의
  기준 버전만 수정.
- **선택과 근거**: 기준 버전만 `0.3.0-dev.6`으로 올린다. 배포 workflow가
  `stamp-version.mjs`로 `tt-lang`과 `create-tt`을 같은 불변 개발 버전으로
  스탬프하는 저장소 계약을 따른다.

## 작업 내역

- 2026-08-24: `Cargo.toml`과 `Cargo.lock`의 ttc 버전을
  `0.3.0-dev.6`으로 올렸다.
- 2026-08-24: Rust 검증 게이트와 npm 릴리스 도구·`create-tt` 테스트를
  통과했다.

## 이슈 및 해결

없음.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `node --test npm/scripts/*.test.mjs packages/create-tt/test/*.test.mjs`

## 결과

`0.3.0-dev.6` 기준 버전을 준비했다. `main` push 후 CI 성공 시 개발 배포
workflow가 플랫폼별 ttc, `@load28/tt-lang`, `@load28/create-tt`,
`@load28/unplugin-tt`과 GitHub VSIX pre-release를 배포한다.
