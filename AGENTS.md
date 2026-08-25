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

버전은 작업 단위로 올리지 않습니다. `Cargo.toml` 버전은 명시적인 릴리스 태스크에서만
변경하며 npm 메인 패키지와 설치기 버전은 배포 스크립트가 스탬프합니다.

## 작업 브랜치와 릴리스

저장소 추적 파일을 바꾸는 작업은 최신 main에서 별도 작업 브랜치를 만든 뒤 진행합니다.
사용자가 명시적으로 main 직접 작업을 요청하지 않은 한 개발 커밋을 main에 직접
push하지 않습니다. 태스크 문서, 구현, 검증 수정은 모두 같은 작업 브랜치에 커밋하고
원격에 push합니다. 버전 파일은 작업 브랜치에서 직접 바꾸지 않습니다.

배포는 사용자가 Dev 또는 Production 릴리스를 명시적으로 요청했을 때 작업자가 GitHub
CLI로 실행하고 완료까지 확인합니다. GitHub Actions 화면에서 사용자가 직접 입력할
필요는 없습니다. 모든 릴리스 워크플로는 `main`의 워크플로 정의로 실행합니다.

### Dev 릴리스

1. 작업 브랜치의 로컬 게이트를 통과시키고 원격에 push합니다.
2. `gh workflow run dev-release.yml --ref main -f source_ref=<작업-브랜치>`를 실행합니다.
   특정 코어 버전이면 `-f version=X.Y.Z`도 전달합니다. 버전이 없으면 최신 릴리스에서
   자동 계산합니다.
3. Prepare 액션은 지정한 ref의 정확한 SHA에서 `release/dev-X.Y.Z-dev.N`을 만들고
   tsgo 검사, 5개 플랫폼 빌드, VSIX 패키징을 수행합니다. 성공한 SHA와 Actions run
   ID를 `release/dev-prepare` 상태로 기록합니다. 이 단계는 게시하지 않습니다.
4. 성공 후 `gh workflow run dev-publish.yml --ref main -f version=X.Y.Z`를 실행합니다.
   Approve 액션은 기록된 run ID의 산출물만 받아 npm `dev`와 GitHub prerelease로
   게시하고 완료된 Dev 릴리스 브랜치를 삭제합니다. Dev는 main에 병합하지 않습니다.

Prepare가 실패하면 게시하지 않고 릴리스 브랜치를 보존합니다. 수정은 릴리스 브랜치나
main에 직접 넣지 않고 원본 작업 브랜치에 커밋·push합니다. 같은 `source_ref`와 코어
버전으로 Prepare를 다시 실행하면 새 작업 SHA를 기존 릴리스 브랜치에 병합하고 같은
Dev 버전을 처음부터 다시 검증합니다. 성공 뒤 작업 브랜치가 바뀌었다면 승인 전에
Prepare를 다시 실행해야 새 커밋이 포함됩니다. 게시가 끝난 뒤의 변경은 다음 Dev 번호로
릴리스합니다. 릴리스 브랜치에 수정 커밋을 cherry-pick하지 않습니다.

### Production 릴리스

1. Production은 성공한 미승격 Dev 태그만 승격합니다. 작업 브랜치는 해당 Dev를 만들기
   전에 최신 main을 포함해야 합니다. main이 Dev 뒤에 전진했다면 main을 작업 브랜치에
   병합하고 새 Dev를 먼저 게시합니다.
2. `gh workflow run release.yml --ref main -f version=X.Y.Z`를 실행합니다. 버전이 없으면
   최신 미승격 Dev를 선택합니다.
3. Prepare 액션은 Dev 태그에서 `release/vX.Y.Z`을 만들고 재검증·플랫폼 빌드한 뒤
   main 대상 Production PR을 엽니다. 이 단계도 게시하지 않습니다.
4. 준비 성공과 PR diff를 확인한 뒤 PR을 main에 병합합니다. 준비된 Production PR의
   병합만 `release-publish.yml`을 자동 실행합니다. 이 액션은 준비 run ID의 바이너리로
   npm `latest`와 GitHub Release를 게시하고 Production 릴리스 브랜치를 삭제합니다.

빌드나 게시가 실패하면 성공으로 보고하지 않습니다. 같은 릴리스 브랜치와 액션을
재사용해 멱등하게 재시도하며, npm의 동일 버전이 다른 SHA에서 이미 게시된 경우에는
중단합니다. Dev 태그는 Production 승인 전 검증 기준이고, main에는 Production PR이
병합될 때 작업 코드와 안정 버전이 함께 들어갑니다.

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

변경 완료 전 게이트를 실행합니다. GitHub Actions의 `CI`는 `workflow_dispatch`
전용이라 push나 PR로 돌지 않습니다 — 검증은 로컬에서 끝나야 하고, `scripts/ci`가
CI 잡을 그대로 재현합니다.

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
