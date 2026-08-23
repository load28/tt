# TASK-185: release 0.3.0-dev.4

- **상태**: 취소
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: 0aed915

## 목적

명시적인 npm 로컬 경로 수정이 포함된 `0.3.0-dev.4` 개발 릴리스를 자동
배포하고 npm 사용자용 패키지와 GitHub VSIX pre-release까지 확인한다.

## 범위

- 포함: Cargo 기준 버전을 `0.3.0-dev.4`로 변경, CI와 자동 Dev Release,
  npm `dev` 패키지와 VSIX 결과 확인
- 제외: production 배포, npm `latest` dist-tag 변경

## 의사결정

### 결정 1: 부분 게시된 dev.3 대신 dev.4 사용

- **상황**: dev.3의 플랫폼 패키지 다섯 개는 immutable registry 버전으로
  게시됐지만 사용자용 패키지에서 작업이 실패했다.
- **검토한 대안**: 실패 작업을 재실행하면 이미 게시된 플랫폼 버전과 충돌한다.
  새 기준 버전은 자동 배포 계약을 그대로 유지하면서 모든 패키지에 새 immutable
  버전을 부여한다.
- **선택과 근거**: Cargo 기준 버전을 `0.3.0-dev.4`로 올려 전체 개발 배포를
  다시 실행한다.

## 작업 내역

- 2026-08-23: TASK-185를 등록하고 Cargo 기준 버전을 `0.3.0-dev.4`로 변경했다.
- 2026-08-23: Node 릴리스 도구 테스트 11건과 세 검증 게이트를 통과했다.

## 이슈 및 해결

### 이슈 1: `unplugin-tt` 이름이 기존 패키지와 유사해 차단됨

- **증상**: Dev Release run `32642837256`에서 `tt-lang`은 게시됐지만
  `unplugin-tt` 게시가 npm 403 `Package name too similar to existing package
  unplugin-dts`로 실패했다.
- **원인**: npm의 신규 언스코프 패키지 유사 이름 보호 정책이 `unplugin-tt`을
  기존 `unplugin-dts`의 유사 이름으로 판정했다.
- **해결**: 이 릴리스는 부분 게시 상태로 취소하고 TASK-186에서 브랜드 우선
  고유 이름으로 변경한다. 이어지는 `create-tt`도 registry 조회 결과 다른 사용자가
  이미 소유하므로 함께 변경한다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## 결과

CI run `32642637756`은 통과했고 플랫폼 패키지 다섯 개와 `tt-lang`도 게시됐다.
`unplugin-tt` 이름 차단으로 후속 게시와 VSIX가 중단됐으므로 dev.4는 후속 개발
버전으로 대체한다.
