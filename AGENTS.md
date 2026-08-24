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

## 구현과 검증 규칙

- Rust MSRV는 `Cargo.toml`의 `rust-version`이며 `unsafe`는 금지합니다.
- 스캐너와 변환기는 ASCII 바이트만 판단하고 멀티바이트 UTF-8은 불투명하게
  통과시킵니다.
- 내부 오류는 바이트 오프셋을 담고 사용자 line/column 변환은 공개 경계에서 합니다.
- 새 기능은 출력 계약이면 `tests/compile.rs`, TS 통과 계약이면
  `tests/passthrough.rs`, 타입·런타임 의미이면 통합 테스트를 추가합니다.
- 기존 사용자 변경을 보존하고 관련 없는 dirty 파일을 수정하지 않습니다.

변경 완료 전 다음 게이트를 모두 실행합니다.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
