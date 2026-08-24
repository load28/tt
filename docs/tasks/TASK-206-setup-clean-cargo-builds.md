# TASK-206: setup의 Cargo 전체 정리와 tsgo 자식 환경 주입

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: —

## 목적

로컬 setup을 실행할 때 이전 디버그·릴리즈 Cargo 산출물이 새 빌드에 섞이지 않도록 한다. checkout setup의 후속 자식 프로세스에는 방금 검증한 tsgo 경로를 명시적으로 전달한다.

## 범위

- 포함: `scripts/setup`의 TT 빌드 직전에 Cargo 산출물 전체 삭제, release 바이너리 재빌드, 현재 setup 셸의 `TTC_TSGO_ROOT`·`TTC_TSGO_BIN`·`TTC_TSGO_API` export, 관련 안내 갱신
- 제외: typescript-go와 VS Code 확장의 의존성 캐시 삭제, setup이 생성하는 최종 바이너리 프로필 변경

## 의사결정

### 결정 1: 프로필별 디렉터리 삭제 대신 `cargo clean`을 사용한다

- **상황**: 기존 `target/debug`와 `target/release`를 모두 제거하되 Cargo가 생성하는 다른 공유 산출물과 사용자 지정 target 디렉터리도 일관되게 처리해야 한다.
- **검토한 대안**: `target/debug`와 `target/release`만 직접 삭제하면 범위가 눈에 보이지만 Cargo의 target 디렉터리 설정과 기타 산출물을 놓친다. `cargo clean`은 Cargo가 관리하는 현재 패키지의 target 디렉터리 전체를 정식 인터페이스로 정리한다.
- **선택과 근거**: TT 루트에서 `cargo clean`을 실행한다. 디버그와 릴리즈를 포함한 기존 Cargo 산출물을 한 번에 제거하며 직접 경로를 삭제하지 않는다.

### 결정 2: 정리 후에는 기존대로 release 프로필만 다시 빌드한다

- **상황**: setup의 로컬 npm 실행기와 요약은 `target/release/ttc`를 계약으로 사용한다.
- **검토한 대안**: debug와 release를 모두 다시 빌드하면 두 프로필을 즉시 제공하지만 setup 시간과 저장 공간이 늘고 사용되지 않는 debug 바이너리를 만든다. release만 빌드하면 기존 실행 계약을 유지한다.
- **선택과 근거**: `cargo clean` 뒤 `cargo build --release`를 유지한다. 사용자의 요청은 두 프로필의 기존 파일 제거로 충족하고 setup의 출력 계약은 바꾸지 않는다.

### 결정 3: 검증된 checkout 경로를 setup 셸에만 export한다

- **상황**: setup이 이어서 실행하는 빌드·패키징 자식 프로세스가 방금 빌드한 typescript-go checkout을 명시적으로 참조해야 한다.
- **검토한 대안**: 셸 프로필이나 클라우드 환경을 영구 변경 / `.tt-dev/toolchain.json`만 사용 / 현재 setup 셸에 export.
- **선택과 근거**: checkout 산출물 검증 직후 세 `TTC_TSGO_*` 변수를 export한다. 현재 setup 셸과 자식 프로세스에만 영향을 주므로 외부 환경을 변경하지 않는다.

## 작업 내역

- 2026-08-24: `scripts/setup`의 TT 빌드 단계 앞에 `cargo clean`을 추가하고, 디버그·릴리즈 산출물이 모두 정리된다는 진행 메시지와 상단 절차 설명을 갱신했다.
- 2026-08-24: checkout 산출물 검증 직후 root·실행 파일·API 경로를 세 `TTC_TSGO_*` 환경 변수로 export하도록 했다.
- 2026-08-24: 기존 `target/debug`와 `target/release`가 모두 있는 상태에서 `cargo clean`을 실행했다. 156,011개·17.2 GiB를 제거했고 두 디렉터리가 사라진 것을 확인한 뒤 release `ttc`를 새로 빌드했다.
- 2026-08-24: setup 구문과 저장소 필수 Rust 게이트를 검증했다.
- 2026-08-24: `docs/tasks/INDEX.md`에 TASK-206을 진행 중으로 등록하고 다음 번호를 TASK-207로 변경했다.

## 이슈 및 해결

없음.

## 검증

- [x] `bash -n scripts/setup`
- [x] clean 뒤 `target/debug`·`target/release` 제거와 release `ttc` 재생성 확인
- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` (827 통과)

## 결과

`scripts/setup`은 TT를 빌드하기 전에 모든 Cargo 프로필 산출물을 정리하고 release
`ttc`를 새로 만든다. checkout 모드에서는 검증한 로컬 tsgo 경로를 현재 setup
셸과 이후 자식 프로세스에 export한다.
