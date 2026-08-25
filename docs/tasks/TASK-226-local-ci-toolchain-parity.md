# TASK-226: 로컬과 CI의 Rust 툴체인 격차

- **상태**: 대기
- **시작일**: —
- **완료일**: —
- **커밋**: —

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

## 작업 내역

## 이슈 및 해결

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] 선택한 방식이 실제로 격차를 드러내는지 확인 (예: 의도적으로 낮은 툴체인에서
      doctor가 경고하는지)

## 결과
