# TASK-226: 로컬과 CI의 Rust 툴체인 격차

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: `d27105a`, `cee2030`

## 목적

PR #53의 `fmt / clippy / test` 잡이 `manual_option_zip` 린트로 실패했다. 로컬에서
`cargo clippy --all-targets -- -D warnings`는 통과한 코드였다. 원인은 툴체인
버전 차이다.

- 로컬: rustc/clippy **1.94.1** (컨테이너에 설치된 것)
- CI: `dtolnay/rust-toolchain@stable` → 그날의 stable, 이 경우 **1.98.0**

`scripts/doctor`는 `Rust $RUST_VERSION satisfies MSRV 1.88`만 확인한다. 즉
**하한만 보고 CI와의 일치는 보지 않는다.** 그 사이에 추가된 clippy 린트는 로컬에서
보이지 않고 CI에서만 빨개진다. 게이트를 로컬에서 먼저 통과시키라는 계약이
있는데도 통과 여부가 환경에 따라 달라지면, 그 계약은 지켜질 수 없다.

이번에는 `rustup update stable`로 맞춘 뒤 로컬에서 재현·수정했지만, 이것이 매번
사람이 기억해야 하는 일로 남으면 같은 일이 반복된다.

## 범위

- 포함: 아래 세 방향의 트레이드오프를 판단하고 하나를 고른다.
  - **(a) doctor가 경고한다** — 로컬 stable이 최신이 아니면 `warning`으로 알리고
    `rustup update`를 안내한다. 강제하지 않으므로 오프라인에서도 막히지 않는다.
    다만 경고는 무시될 수 있다.
  - **(b) `rust-toolchain.toml`로 고정** — 로컬과 CI가 같은 버전을 쓴다. 가장
    확실하지만, 새 린트를 늦게 발견하고 고정 버전을 올리는 태스크가 주기적으로
    필요하다.
  - **(c) CI를 고정 버전으로** — (b)와 같은 효과. CI가 `stable`인 한 남의 릴리스가
    남의 커밋에서 빨개질 수 있다는 점은 typescript-go ref를 고정한 것과 같은
    논점이다(`.github/workflows/ci.yml`의 "Bumping it is a task, not a side
    effect").
- 제외:
  - 문제가 된 린트에 `#[allow]`를 다는 것. 그것은 이 격차를 덮을 뿐이다.
  - MSRV(1.88) 변경. MSRV는 소비자가 쓸 수 있는 최소 버전이고, 개발 툴체인과 다른
    축이다.

## 의사결정

### 결정 1: 세 방향 중 `rust-toolchain.toml` 고정

- **상황**: 범위에 적어 둔 (a) doctor 경고 / (b) 파일 고정 / (c) CI 버전 고정 중
  하나를 골라야 했다.
- **검토한 대안**:
  - (a)는 결국 "무엇과 비교해 경고하느냐"를 답해야 하고, 그 기준값을 저장소에
    적는 순간 (b)가 된다. 게다가 경고는 무시될 수 있다.
  - (c)는 CI만 고정하므로 로컬은 여전히 제각각이다. 격차 자체가 문제인데 한쪽만
    고정하면 격차는 남는다.
  - (b)는 rustup이 이 저장소에서 그 버전을 **자동으로 선택**한다. 기여자가 아무것도
    기억하지 않아도 로컬과 CI가 같아진다.
- **선택과 근거**: (b). 그리고 CI의 세 잡에서 `dtolnay/rust-toolchain@stable`을
  없애고 `rustup show active-toolchain`으로 대체했다 — 버전을 말하는 곳이
  `rust-toolchain.toml` 하나뿐이어야 드리프트가 없고, 그 명령은 없으면 설치하고
  어느 파일이 골랐는지까지 로그에 남긴다.

### 결정 2: 액션의 동작을 추측하지 않고 확인했다

- **상황**: 파일을 추가했을 때 CI가 실제로 그 버전을 쓰는지는 rustup의 우선순위와
  액션의 선택 방식에 달려 있다. 둘 다 추측하면 "고쳤다고 믿는데 안 고쳐진" 상태가
  된다.
- **확인 방법과 결과**:
  - rustup 우선순위를 실제로 측정: `rust-toolchain.toml`(1.98.0)이 있는 디렉터리에서
    `RUSTUP_TOOLCHAIN=1.88.0`과 `+1.88.0`은 **둘 다 파일을 이겼다**. 파일은
    `rustup default`는 이긴다.
  - `dtolnay/rust-toolchain`의 `action.yml`을 받아 읽음:
    `rustup toolchain install` 후 **`rustup default <toolchain>`**을 실행한다.
    `RUSTUP_TOOLCHAIN`을 쓰지 않는다.
  - 파일의 `components`가 첫 cargo 호출에서 자동 설치되는지 확인: 설치된다
    (`info: downloading component clippy`).
  - `rustup show active-toolchain`이 미설치 버전을 설치하는지 확인: 설치하고
    출처까지 출력한다.
- **함의**: 파일만 추가하면 액션이 세운 default를 파일이 덮으므로 **MSRV 잡이
  조용히 1.98로 빌드하게 된다** — MSRV 검증이 사라지는데 CI는 초록인 상태. 그래서
  처음에는 그 잡만 `cargo +<msrv>`로 예외를 뒀다(결정 3에서 뒤집힘).

### 결정 3: 모든 잡을 핀 하나로 — MSRV 잡 제거

- **상황**: 결정 2의 예외 처리는 "핀을 쓰지 않는 유일한 잡"을 만든다.
- **선택과 근거**: 사용자 지시("툴체인을 1.98로 설정했으니 이걸로 다 써야 한다")에
  따라 예외를 없애고 `build on MSRV` 잡을 제거했다. CI는 `check`·`extension`·
  `native` 세 잡이며 모두 핀을 쓴다.
- **뒤따른 문제**: 잡을 없애면 `Cargo.toml`의 `rust-version = "1.88"`과 README
  영문·한글의 "Rust 1.88 or newer" 문구를 아무것도 검증하지 않게 된다. 1.88 이후에
  들어온 기능을 쓰면 CI는 초록인데 1.88 사용자는 알 수 없는 컴파일 에러를 본다.
  → 결정 4.

### 결정 4: 선언을 실제와 맞춘다 — `rust-version`을 1.98로

- **상황**: 결정 3 이후 "선언된 최소 버전"과 "실제로 컴파일해 보는 버전"이 어긋났다.
- **검토한 대안**:
  - **(a)** `rust-version`을 1.98로 올린다. 선언과 실제가 일치한다. 대신 1.88~1.97
    사용자는 더 이상 쓸 수 없다.
  - **(b)** MSRV 잡을 되살린다. 1.88 약속을 계속 검증하지만, "핀을 쓰지 않는 유일한
    잡"이라는 예외가 돌아온다.
  - (c) 그대로 둔다 — 선언은 남고 확인만 없는 상태. 셋 중 가장 나쁘다: 아무도
    검증하지 않는 약속은 어긴 줄도 모르고 어기게 된다.
- **선택과 근거**: (a). 사용자 결정이며, tt이 아직 `0.3.0-dev`이라 지원 하한을
  올리는 비용이 작다는 점이 근거다. 무엇보다 이 태스크의 주제 자체가 "확인되지 않는
  기준은 기준이 아니다"였으므로, 개발 툴체인만 고정하고 소비자 하한은 확인 없이
  남겨 두는 것은 같은 실수의 반복이 된다. 이제 한 버전이 개발·CI·소비자 하한
  셋 모두를 뜻하고, 핀을 올리면 소비자 요구도 함께 올라간다 — 핀 상향이 태스크여야
  하는 이유가 하나 더 늘었다.
- **함께 바꾼 것**: `Cargo.toml`의 `rust-version`, `README.md`, `README.ko.md`,
  그리고 `rust-toolchain.toml`의 주석.

## 작업 내역

- 2026-08-25: rustup 우선순위와 액션 동작을 실측(결정 2).
- 2026-08-25: `rust-toolchain.toml` 추가 — `channel = "1.98.0"`,
  `components = ["rustfmt", "clippy"]`. 왜 고정하는지, 올리는 것이 태스크인 이유,
  MSRV와 다른 축이라는 점을 파일 주석에 남겼다.
- 2026-08-25: CI의 세 잡에서 `dtolnay/rust-toolchain@stable` 제거,
  `rustup show active-toolchain`으로 대체. `build on MSRV` 잡 제거(결정 3).
- 2026-08-25: `scripts/doctor`가 MSRV 하한 대신 **활성 툴체인이 핀과 같은지**
  확인하도록 변경. `MIN_RUST_MAJOR/MINOR` 상수와 `rust_is_supported()` 제거,
  `pinned_rust()` 추가. 이제 `ok: Rust 1.98.0 matches rust-toolchain.toml`.
- 2026-08-25: `AGENTS.md`, `CONTRIBUTING.md`에 핀과 `rust-version`의 관계를 적었다.
- 2026-08-25: 결정 4에 따라 `rust-version`을 1.98로 올리고 README 영문·한글과
  `rust-toolchain.toml` 주석을 맞췄다. 1.88을 언급하던 곳은 저장소 전체에서
  세 군데였고(`Cargo.toml`, 두 README) 모두 갱신했다.

## 이슈 및 해결

없음. (결정 3의 미해결 항목 참조)

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings` — 경고 0
- [x] `cargo test` — 14개 스위트 전부 통과, skip 없음
- [x] `rustup show active-toolchain` →
      `1.98.0-x86_64-unknown-linux-gnu (overridden by '.../rust-toolchain.toml')`
- [x] `./scripts/doctor` → `ok: Rust 1.98.0 matches rust-toolchain.toml`
- [x] `.github/workflows/ci.yml`이 유효한 YAML이고 남은 세 잡이 모두 핀을 쓰는지
      파싱해서 확인

## 결과

### 변경된 파일

- `rust-toolchain.toml` (신규) — 개발 툴체인 고정
- `.github/workflows/ci.yml` — 세 잡이 핀을 쓰도록, `build on MSRV` 잡 제거
- `scripts/doctor` — MSRV 하한 검사를 핀 일치 검사로 교체
- `Cargo.toml`, `README.md`, `README.ko.md` — 선언된 최소 버전을 1.98로
- `AGENTS.md`, `CONTRIBUTING.md` — 핀과 `rust-version`의 관계

### 후속

- 핀 상향은 별도 태스크로 한다. 이제 소비자 하한까지 함께 올라가므로 더욱 그렇다.
