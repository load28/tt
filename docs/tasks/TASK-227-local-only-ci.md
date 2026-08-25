# TASK-227: CI를 로컬 실행으로 옮기고 GitHub 실행은 수동으로

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: —

## 목적

이 저장소 소유자는 GitHub 무료 플랜이라 Actions 실행 한도가 빠듯하다. 현재
`CI` 워크플로는 `push: main`과 `pull_request` 모두에서 자동으로 돌고, 잡 세 개
중 둘(`extension`, `native`)은 typescript-go를 통째로 빌드하거나 npm 설치를
반복하는 무거운 잡이다. 브랜치에 push할 때마다, PR을 열 때마다, PR에 커밋을
얹을 때마다 이 비용이 나간다.

게이트 자체를 없애는 것은 답이 아니다. 검증은 그대로 두되 **실행 장소를 로컬로
옮기고**, 호스팅된 실행은 필요한 순간에만 사람이 시작하게 한다.

## 범위

- 포함:
  - `.github/workflows/ci.yml`의 자동 트리거(`push: main`, `pull_request`)를
    제거하고 `workflow_dispatch`만 남긴다.
  - CI 잡이 하던 검증을 로컬에서 그대로 재현하는 `scripts/ci`를 추가한다.
  - 릴리스 워크플로가 "main의 CI 성공"에 의존한다는 사실과, 이제 그 성공을
    사람이 만들어야 한다는 점을 `CONTRIBUTING.md`에 적는다.
  - `AGENTS.md`의 검증 게이트를 `scripts/ci` 기준으로 갱신한다.
- 제외:
  - 잡이나 테스트를 줄이는 것. 비용을 줄이는 수단은 **실행 장소**이지 검증
    범위가 아니다.
  - `pages.yml`, `release.yml`, `dev-release.yml`의 트리거 변경. 앞의 둘은
    실제 배포 행위이고 자주 돌지 않는다.
  - pre-push 훅 설치. 사용자가 스크립트 형태를 선택했다.

## 의사결정

### 결정 1: 워크플로를 지우지 않고 `workflow_dispatch`만 남긴다

- **상황**: "PR에서 안 돌게" 하는 방법이 여러 가지다.
- **검토한 대안**:
  - **(a) `ci.yml` 삭제** — 확실하지만 `release.yml`과 `dev-release.yml`이
    `workflow_run: workflows: [CI]`로 CI 성공을 기다린다. 지우면 두 배포
    경로의 트리거를 함께 재설계해야 한다. 이 태스크의 목적(비용)과 관계없는
    범위 확장이다.
  - **(b) `pull_request`만 제거하고 `push: main` 유지** — 릴리스 배선은
    그대로지만 main으로 가는 모든 커밋이 여전히 세 잡을 돌린다. 사용자는
    "로컬에서만"이라고 했다.
  - **(c) `workflow_dispatch`만 남긴다** — 자동 실행이 0이 된다. 그러면서
    `workflow_run`은 무엇이 CI를 시작했는지 가리지 않으므로, main에서 CI를
    수동으로 한 번 돌리면 릴리스 파이프라인은 지금 그대로 동작한다.
- **선택과 근거**: (c). 사용자 선택이며, 배선을 건드리지 않고 자동 소모만
  0으로 만드는 유일한 대안이다.

### 결정 2: 로컬 게이트를 문서가 아니라 스크립트로 둔다

- **상황**: 로컬에서 무엇을 돌려야 CI와 같은지가 문제다. 지금까지 문서에 적힌
  게이트는 `cargo fmt --check`, `cargo clippy`, `cargo test` 셋뿐이었는데,
  CI는 그 밖에도 에이전트 진입점 계약, npm 릴리스 도구 테스트, VS Code 확장
  테스트, 그리고 **스킵 금지 가드**를 강제하고 있었다.
- **검토한 대안**: 문서에 명령을 나열하기 / 실행 스크립트를 두기.
- **선택과 근거**: 스크립트. 나열된 명령은 옮겨 적는 순간 드리프트가 시작되고,
  무엇보다 CI가 강제하던 것 중 사람이 손으로 재현하기 어려운 것들 — 확장
  테스트 로그에서 스킵 문자열을 찾아내는 가드, `TTC_REQUIRE_TSGO=1` 주입,
  `.tt-dev/toolchain.json`에서 tsgo 경로를 꺼내 자식 프로세스에 넣는 일 — 이
  전부 생략될 것이다. 자동 실행을 없앤 자리에 "사람이 기억해야 하는 목록"을
  놓으면 게이트는 사실상 사라진다.

### 결정 3: 도구가 없어서 생기는 스킵은 경고로 드러낸다

- **상황**: CI는 `tsc`, `rolldown`, TypeScript 7을 **설치한 다음** 테스트를
  돌렸다. 로컬에는 없을 수 있고, 없으면 스위트는 초록인 채로 조용히 스킵된다.
  저장소가 이미 CI에 스킵 금지 가드를 둔 이유가 그것이다.
- **선택과 근거**: `scripts/ci`는 시작할 때 도구를 점검해 **무엇이 없으면 어떤
  검증이 사라지는지**를 이름으로 경고하고, 그 경고를 마지막 요약에서 한 번 더
  낸다. `native` 단계는 경고가 아니라 실패다 — 그 단계의 존재 이유가
  TypeScript 7 경로를 실제로 도는 것이고, 없는 채로 통과시키면 CI에 두었던
  가드를 로컬에서 되돌리는 셈이기 때문이다.

## 작업 내역

- 2026-08-25: `ci.yml` 트리거를 `workflow_dispatch`로 교체하고, 왜 수동인지와
  릴리스 파이프라인이 이 워크플로의 성공에 의존한다는 점을 파일 상단 주석에
  남겼다.
- 2026-08-25: `scripts/ci` 추가. `ci.yml`의 세 잡을 다섯 단계
  (`agents`/`rust`/`npm`/`native`/`extension`)로 옮기고, 단계 선택·건너뛰기와
  도구 점검, 스킵 금지 가드를 구현했다.
- 2026-08-25: `CONTRIBUTING.md`에 로컬 게이트와 수동 CI 실행 절차를,
  `AGENTS.md`에 `./scripts/ci`를 적었다.

## 이슈 및 해결

없음.

## 검증

- [x] `./scripts/ci --list`, `--help`, 단계 선택, `--skip`, 알 수 없는 단계 이름
      (exit 2), 모두 건너뛴 경우(exit 2)를 실제로 실행해 확인
- [x] `./scripts/ci agents` — 통과. doctor가 작업 트리를 바꾸지 않고, 준비되지
      않은 체크아웃은 경고로 드러난다
- [x] `./scripts/ci native` — 툴체인이 없는 이 컨테이너에서 의도대로 **실패**하고
      (exit 1) 두 가지 해결책을 이름으로 안내한다
- [x] `./scripts/ci rust npm` — `npm` 통과(21 tests, 0 skipped),
      `cargo fmt --check`·`cargo clippy --all-targets -- -D warnings` 통과
- [ ] `cargo test` — **실패**. `tests/engine_cache.rs`의
      `an_error_node_keeps_its_file_and_other_files_checkable` 하나. 이 태스크는
      Rust 코드를 한 줄도 바꾸지 않았고, 병합 커밋 `d46d097`에서 3회 연속 동일하게
      실패하는 기존 결함이다. TASK-228로 등록했다.
- [x] `.github/workflows/*.yml` 네 개를 파싱해 트리거 확인 —
      `CI`는 `workflow_dispatch` 하나, `Release`/`Dev Release`의
      `workflow_run: [CI] on main` 배선은 그대로다

## 결과

### 변경된 파일

- `.github/workflows/ci.yml` — 트리거를 `workflow_dispatch` 하나로. 왜 수동인지와
  릴리스 두 워크플로가 이 워크플로의 성공에 의존한다는 점을 상단 주석에 남겼다
- `scripts/ci` (신규) — CI 잡을 다섯 단계로 재현하는 로컬 게이트
- `CONTRIBUTING.md` — "머지 전 검증 게이트"를 `scripts/ci` 기준으로 다시 쓰고,
  개발/정식 배포 절차에 "main에서 CI 수동 실행" 단계를 추가
- `AGENTS.md` — 검증 게이트를 `./scripts/ci`로
- `docs/tasks/INDEX.md`, `docs/tasks/TASK-227-*.md`, `docs/tasks/TASK-228-*.md`

### 자동 실행이 사라지면서 사람이 해야 하는 일

- 머지 전: `./scripts/ci`. 경고가 붙은 통과는 CI와 같은 통과가 아니다.
- 배포 전: `main`에서 Actions → `CI` → **Run workflow**. 그 실행이 성공해야
  `Release`/`Dev Release`가 깨어난다. `vX.Y.Z` 태그 경로는 CI와 무관하다.

### 후속

- [TASK-228](./TASK-228-partial-snapshot-diagnostics.md) — 이 태스크 도중 드러난
  기존 `cargo test` 실패. 게이트를 로컬로 옮긴 첫 실행에서 잡혔다.
