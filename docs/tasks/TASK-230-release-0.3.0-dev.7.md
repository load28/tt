# TASK-230: release 0.3.0-dev.7

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: `2c94b31`, `928dbc3`

## 목적

TASK-194부터 TASK-229까지 완료된 변경을 npm `dev` 채널과 GitHub VSIX
개발 채널로 배포한다.

## 범위

- 포함: Cargo 기준 버전을 `0.3.0-dev.7`로 변경, 검증, `main` push,
  수동 Dev Release 실행과 게시 결과 확인
- 제외: production 배포와 npm `latest` dist-tag 변경

## 의사결정

### 결정 1: 기존 개발 버전의 다음 순번 사용

- **상황**: 현재 기준 버전 `0.3.0-dev.6` 이후 변경을 불변 npm 버전으로
  게시하려면 새 개발 버전이 필요하다.
- **검토한 대안**: stable 버전 배포 / 개발 순번 증가.
- **선택과 근거**: 요청한 dev 배포 계약에 따라 `0.3.0-dev.7`을 사용한다.
  자동 배포 workflow가 Cargo 기준 버전 증가를 감지하고 타임스탬프가 포함된
  npm 버전과 VSIX 버전을 파생한다.

### 결정 2: Dev Release workflow 수동 실행

- **상황**: CI workflow가 수동 실행 전용으로 변경되어 기존 `workflow_run`
  트리거로는 버전 push 뒤 개발 배포가 시작되지 않는다.
- **검토한 대안**: `Cargo.toml` push 직접 트리거 복원 / Dev Release 수동 실행.
- **선택과 근거**: CI를 수동으로 둔 현재 운영 정책을 유지하고 이번 배포는
  `workflow_dispatch`로 실행한다.

## 작업 내역

- 2026-08-25: `Cargo.toml`과 `Cargo.lock`의 ttc 버전을
  `0.3.0-dev.7`로 올렸다.
- 2026-08-25: Rust 검증 게이트와 npm 릴리스 도구·`create-tt` 테스트를
  통과했다.
- 2026-08-25: 자동 트리거를 변경하지 않고 Dev Release를 수동 실행하기로 했다.
- 2026-08-25: Dev Release run `32855328176`의 크로스 타깃 빌드 실패를
  확인하고 두 릴리스 workflow가 고정 toolchain에 타깃을 설치하도록 수정했다.
- 2026-08-25: Dev Release run `32855777128`이 성공했고 npm 패키지 8개와
  VSIX pre-release의 공개 상태를 확인했다.

## 이슈 및 해결

### 이슈 1: 원격 main 갱신으로 최초 push 거절

- **증상**: 최초 `git push origin main`이 non-fast-forward로 거절됐다.
- **원인**: 작업 중 원격에 TASK-213부터 TASK-229까지의 변경이 병합됐다.
- **해결**: 원격 변경 위로 릴리스 커밋을 rebase하고 태스크 번호를 TASK-230으로
  조정했다.

### 이슈 2: 고정 toolchain에 크로스 컴파일 타깃 누락

- **증상**: Dev Release run `32855328176`에서 linux-x64, linux-arm64,
  darwin-x64 빌드가 `can't find crate for std`로 실패했다.
- **원인**: `dtolnay/rust-toolchain@stable`이 타깃을 stable toolchain에
  설치했지만 저장소의 `rust-toolchain.toml` 때문에 Cargo는 고정된 1.98을 썼다.
- **해결**: 개발·정식 릴리스 workflow가 `rustup target add`로 활성 고정
  toolchain에도 matrix 타깃을 설치하게 했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `node --test npm/scripts/*.test.mjs packages/create-tt/test/*.test.mjs`
- [x] VS Code 확장 빌드·테스트 114건
- [x] GitHub Actions Dev Release run `32855777128`
- [x] npm `dev` dist-tag와 GitHub VSIX pre-release

## 결과

`0.3.0-dev.7.20260825.135039.30.1` 개발 패키지 8개를 npm `dev` 태그로
게시했다. GitHub pre-release에 `tt-language-0.260825.135039.vsix`를 첨부했다.
