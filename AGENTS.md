# AGENTS.md — tt 프로젝트 작업 가이드

이 파일은 Claude Code와 Codex가 공유하는 저장소 작업 계약입니다. 사람을 위한 상세
환경 설명은 `CONTRIBUTING.md`, tt 사용자 프로젝트용 언어 컨텍스트는
`docs/ai/tt.md`에 있습니다.

## 작업 시작 전

1. 저장소 루트에서 `./scripts/doctor`를 실행해 로컬 환경을 읽기 전용으로 진단합니다.
2. doctor가 성공하면 setup을 다시 실행하지 않습니다.
3. doctor가 toolchain 미설정을 보고하면 `--tsgo-root <path>`와 `--tsgo-npm` 중
   어느 방식을 쓸지 추측하지 말고 사용자에게 확인합니다.
4. doctor가 재구성을 요구하더라도 `./scripts/setup`은 Cargo 산출물을 정리하고
   release `ttc`와 VS Code 확장을 다시 설치하므로 사용자가 로컬 setup 또는
   재설치를 요청한 경우에만 실행합니다.
5. 저장소 추적 파일을 바꾸는 개발 작업은 아래 태스크 문서를 먼저 만듭니다.
   doctor 실행, 기존 setup 실행, 설치 상태 확인만 하는 운영 작업은 태스크에서
   제외합니다.

첫 setup은 다음 중 사용자가 선택한 하나만 실행합니다. 이후에는 인자 없는 명령이
`.tt-dev/toolchain.json`을 재사용합니다.

```sh
./scripts/setup --tsgo-root /path/to/typescript-go
./scripts/setup --tsgo-npm
./scripts/setup
```

## 프로젝트와 설계 계약

**tt**은 TypeScript로 컴파일되는 초경량 전처리 언어이고, **ttc**는 Rust
컴파일러입니다. tt은 Rust 스타일 `enum`, `match`, `try`, `let-else`, `if let`,
`|>`/`flow`, `result` 블록과 `val` 바인딩 수식자를 TypeScript에 추가합니다.

다음 세 계약은 기능·수정·리팩터링에서 깨뜨릴 수 없습니다.

1. **모든 유효한 TypeScript 파일은 그대로 유효한 `.tt` 파일입니다.** 완전히
   인식된 tt 구문만 변환하고 나머지는 바이트 단위로 통과시킵니다. 유일한 예외는
   상대 `.tt`/`.ttx` import 지정자 재작성이며 `--rewrite-imports off`로 끌 수
   있습니다.
2. **에러 계층을 분리합니다.** tt 수준 오류는 ttc가 원본 위치와 함께 보고합니다.
   방출 코드는 타입 트릭 없는 순수 TypeScript여야 하며, 사용자 TypeScript의 타입
   오류는 TypeScript가 담당합니다.
3. **해결은 책임 있는 컴파일러 계층에 일반화해 구현합니다.** 특정 테스트나 문자열
   모양을 겨냥한 분기, 휴리스틱, 진단 억제 또는 폴백으로 문제를 덮지 않습니다.
   문법·AST·HIR·이름 해석·의미 분석·타입 시스템·백엔드 중 책임 있는 모델과
   계약으로 표현하고 구조적으로 같은 입력에 적용하며 회귀 테스트로 고정합니다.

## 아키텍처 경계

- `src/main.rs`, `src/server.rs`: CLI와 JSON-lines 서버 진입점
- `src/lib.rs`, `src/engine/`: 공개 API와 Project/Snapshot 기반 typed engine
- `src/lexer.rs`, `src/parser/`, `src/ast.rs`, `src/hir/`: 구문 인식과 단계 간 계약
- `src/resolve/`, `src/analysis/`, `src/sema.rs`, `src/val.rs`: 이름·패턴·의미 분석
- `src/typescript/`: TypeScript backend seam과 tsgo 도달 방법
- `src/codegen/`: source-preserving TypeScript 방출과 매핑
- `tests/`: 출력, 통과, 타입·런타임 통합 계약

typed 경로는 `Engine::open_project` → `Project` → 불변 `Snapshot` 흐름을 사용합니다.
CLI·에디터·서버는 모두 engine 소비자이고 tsgo 개념은 `src/typescript/` 밖으로
새면 안 됩니다. 더 자세한 현재 구조는 `docs/design/compiler-architecture.md`와
관련 설계 문서를 확인합니다.

새 구문은 AST·parser·HIR·codegen, 새 의미 검사는 resolve·analysis·sema, 방출
형태는 codegen처럼 책임 있는 단계에만 둡니다. 언어 표면을 바꾸면
`docs/ai/tt.md`도 갱신하고, 사용자가 처음 접하는 기능이면 영문·한글 README도
같이 갱신합니다.

## 태스크 관리

모든 개발 작업은 `docs/tasks/INDEX.md`와 개별 `docs/tasks/TASK-NNN-<slug>.md`로
관리합니다.

1. 변경 전에 INDEX의 다음 번호와 `docs/tasks/TEMPLATE.md`로 문서를 만들고
   `진행 중`으로 등록합니다.
2. 선택한 대안과 근거, 시간순 작업 내역, 문제의 증상·원인·해결을 기록합니다.
3. 완료 전에 검증 결과와 변경 파일을 기록하고 INDEX와 문서를 `완료`로 바꿉니다.
4. 커밋 제목은 `TASK-NNN: subject`로 시작합니다.

버전은 작업 단위로 올리지 않습니다. `main`의 Nightly 버전은 예약 CI가 산출물에만
날짜로 스탬프합니다. RC 이후 버전은 `release-X.Y`에서만 릴리스 액션이 변경합니다.

## 작업 브랜치와 릴리스

Microsoft TypeScript와 같이 `main`을 유일한 개발 기준으로 사용합니다. 저장소 추적
파일을 바꾸는 작업은 최신 `main`에서 작업 브랜치를 만들고, 로컬 게이트와 리뷰를 거친
PR을 `main`에 squash merge합니다. `main`과 `release-X.Y`는 항상 빌드 가능한 상태여야
하며 직접 기능 작업을 넣지 않습니다.

CI는 `main`·`release-X.Y` push와 이들을 대상으로 하는 PR에서 자동 실행합니다.
push CI는 정확한 커밋의 5개 플랫폼 바이너리, VSIX, 버전·SHA 메타데이터를 30일간
보관합니다. 게시 액션은 다시 빌드하지 않고 성공한 CI run ID의 산출물만 사용합니다.
Nightly는 예약 CI 성공 후 자동 게시합니다. 정식 릴리스는 성공한 `release-X.Y` CI가
`production` Environment 승인을 만들며, 승인자가 `Approve and deploy`한 뒤 게시합니다.
run ID와 npm tag는 CI 메타데이터에서 자동으로 선택하므로 직접 입력하지 않습니다.
릴리스 액션은 TypeScript와 같이 전용 `tt-release-automation` GitHub App 신원으로
버전 커밋을 push합니다. 이 push가 후속 CI를 자동으로 시작하므로 CI를 별도로 dispatch하지
않습니다. App private key는 Azure Key Vault 대신 `RELEASE_APP_PRIVATE_KEY` Actions
Secret에 보관하고 App ID는 `RELEASE_APP_ID` Actions Variable에 둡니다.

### Nightly

Nightly는 릴리스 브랜치를 만들지 않습니다. 매일 예약된 `main` CI가 소스 버전을
바꾸지 않은 채 산출물에 `X.Y.Z-dev.YYYYMMDD`를 스탬프합니다. CI가 성공하면 게시
워크플로가 그 run ID의 산출물을 npm `next`와 GitHub prerelease로 자동 승격합니다.

### RC·Stable·Patch

TypeScript의 릴리스 브랜치 모델에서 Beta만 생략합니다. `X.Y` RC는 최신 `main`에서
`release-X.Y`를 만들고 `X.Y.0-rc`로 시작합니다. Stable은 `X.Y.0`, 이후 Patch는
`X.Y.1`부터 하나씩 올립니다.

```sh
gh workflow run release.yml --ref main -f line=X.Y -f stage=rc
gh workflow run release.yml --ref main -f line=X.Y -f stage=stable
gh workflow run release.yml --ref main -f line=X.Y -f stage=patch
```

각 push CI가 성공하면 게시 워크플로가 해당 run ID와 `rc`, `latest` 중 맞는 tag를
자동으로 선택하고 `production` Environment에서 대기합니다. 승인자는 최신 후보의
`Approve and deploy`만 누릅니다. RC 뒤의 `main`은 다음 minor 개발을 계속하며, 현재
릴리스에 꼭 필요한 수정만 작업 PR을 `main`에 squash merge한 뒤 `release-X.Y`에
cherry-pick합니다. `release-X.Y`는 Stable 뒤에도 Patch용으로 삭제하지 않습니다.

빌드 실패는 해당 브랜치에 새 수정 커밋을 넣어 CI를 다시 실행합니다. 승인 뒤 게시
직전에 해당 브랜치의 최신 CI인지 다시 확인하므로 오래된 후보는 게시되지 않습니다.
게시 실패는 해당 게시 job을 재실행합니다. npm의 동일 버전이 다른 SHA에서 이미
게시됐으면 중단합니다.

## 구현과 검증 규칙

- 개발 툴체인은 `rust-toolchain.toml`이 고정합니다. rustup이 이 저장소에서 그
  버전을 자동으로 고르므로 따로 할 일은 없고, `./scripts/doctor`가 활성 버전이
  핀과 같은지 확인합니다. 로컬과 CI가 같은 clippy를 써야 "게이트를 먼저
  통과시켜라"가 성립합니다 — 핀을 올리는 것은 부수 효과가 아니라 태스크입니다.
- `Cargo.toml`의 `rust-version`은 소비자에게 필요한 최소 버전 선언이고, 위 핀과
  같은 값으로 유지합니다 — 아무것도 컴파일해 보지 않는 최소 버전 선언은 확인 없는
  약속이기 때문입니다. `unsafe`는 금지합니다.
- 스캐너와 변환기는 ASCII 바이트만 판단하고 멀티바이트 UTF-8은 불투명하게
  통과시킵니다.
- 내부 오류는 바이트 오프셋을 담고 사용자 line/column 변환은 공개 경계에서 합니다.
- 새 기능은 출력 계약이면 `tests/compile.rs`, TS 통과 계약이면
  `tests/passthrough.rs`, 타입·런타임 의미이면 통합 테스트를 추가합니다.
  방출된 TypeScript나 렌더된 진단처럼 **산출물 전체**가 계약인 것은
  `tests/fixtures/` 스냅샷으로 고정하고(`UPDATE_EXPECT=1 cargo test --test
  snapshot`), 갱신된 diff를 읽고 검토합니다.
- 기존 사용자 변경을 보존하고 관련 없는 dirty 파일을 수정하지 않습니다.

변경 완료 전 로컬 게이트를 실행합니다. GitHub Actions의 `CI`도 `main`과
`release-X.Y` 대상 PR·push에서 같은 계약을 자동 검증하지만, 원격 CI는 로컬 검증을
대신하지 않습니다.

```sh
./scripts/ci
```

Rust만 건드린 변경이라면 해당 단계만 돌려도 됩니다(`./scripts/ci rust`). 단계
목록과 도구가 없을 때 무엇이 스킵되는지는 `CONTRIBUTING.md`의 "머지 전 검증
게이트"에 있습니다. 어느 경로로 돌리든 최소한 다음 셋은 통과해야 합니다.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
