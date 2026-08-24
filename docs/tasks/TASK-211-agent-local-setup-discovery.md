# TASK-211: AI 에이전트의 로컬 개발 환경 탐색 표준화

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: `874e103`

## 목적

Claude Code와 Codex가 저장소를 처음 열었을 때 서로 다른 지침을 읽거나 무거운
setup을 추측 실행하지 않도록, 공통 진입 문서와 읽기 전용 환경 진단 경로를 만든다.

## 범위

- 포함: `AGENTS.md` 공통 원본화, `CLAUDE.md` 어댑터화, 읽기 전용
  `scripts/doctor`, 기여 문서 연결, CI 정합성 검증
- 제외: toolchain 자동 설치, `scripts/setup`의 빌드·재설치 동작 변경,
  소비자 프로젝트용 `docs/ai/tt.md` 변경

## 의사결정

### 결정 1: `AGENTS.md`를 공통 원본으로 사용

- **상황**: Codex는 `AGENTS.md`, Claude Code는 `CLAUDE.md`를 자동으로 읽는데 두
  파일이 독립 복사본이라 아키텍처와 작업 규칙이 이미 달라졌다.
- **검토한 대안**: 두 파일을 계속 동기화하면 각 도구의 자동 탐색은 단순하지만
  수동 복사 누락이 재발한다. 별도 공통 문서를 두면 두 진입 파일 모두 간접 참조가
  되어 Codex가 추가 파일을 반드시 읽는다는 보장이 약해진다.
- **선택과 근거**: Codex가 직접 읽는 `AGENTS.md`를 원본으로 두고 Claude가 지원하는
  `@AGENTS.md` import를 `CLAUDE.md`에 둔다. 내용은 한 곳에서만 유지한다.

### 결정 2: 환경 탐색과 환경 구성을 분리

- **상황**: 기존 `scripts/setup`은 Cargo 산출물 정리, release 빌드, VS Code 확장
  재설치까지 수행하므로 에이전트가 상태 확인 목적으로 실행하기에는 부작용이 크다.
- **검토한 대안**: setup에 dry-run을 추가하면 한 파일에서 관리할 수 있지만 빌드
  절차와 진단 책임이 결합된다. 문서만 보강하면 실제 머신 상태와 문서가 어긋날 수 있다.
- **선택과 근거**: 파일을 쓰지 않는 `scripts/doctor`를 별도 제공한다. doctor는
  도구·저장된 toolchain·핵심 산출물을 검사하고 필요한 다음 명령만 출력한다.

### 결정 3: 진입점 계약을 CI에서 기계적으로 검사

- **상황**: 문서 구조는 Rust 테스트가 검증하지 않으므로 이후 다시 복제본으로
  돌아가거나 doctor가 문법 오류를 가져도 기존 게이트가 잡지 못한다.
- **검토한 대안**: 리뷰 규칙만 두면 자동 강제가 없다. 별도 테스트 프레임워크는
  단순한 파일 계약에 비해 비용이 크다.
- **선택과 근거**: CI에서 `CLAUDE.md` import 형태, doctor 셸 문법, doctor의
  비변경 진단 실행을 작은 셸 게이트로 검증한다.

## 작업 내역

- 2026-08-25: 자동 진입 문서, 기여 문서, setup 동작과 현재 로컬 toolchain 상태를 조사했다.
- 2026-08-25: `AGENTS.md`를 setup 우선 공통 지침으로 갱신하고 `CLAUDE.md`를
  `@AGENTS.md` import 하나로 축소했다.
- 2026-08-25: 도구 버전, toolchain 설정과 checkout 산출물, 실행 가능한 release
  compiler, 로컬 npm 연결, VSIX를 읽기 전용으로 검사하는 `scripts/doctor`를 추가했다.
- 2026-08-25: `CONTRIBUTING.md`와 태스크 INDEX가 공통 진입점과 doctor를 가리키도록
  바꾸고, CI에 import·셸 문법·무변경 실행 계약을 추가했다.
- 2026-08-25: 구성된 현재 체크아웃과 구성이 없는 임시 체크아웃에서 doctor의 성공·실패
  경로를 각각 실행하고, CI YAML 파싱과 저장소 검증 게이트를 확인했다.

## 이슈 및 해결

### 이슈 1: 한 apply patch에서 같은 파일의 삭제와 추가가 거부됨

- **증상**: `apply_patch verification failed: invalid patch: multiple operations target AGENTS.md`
  오류로 첫 일괄 패치가 적용되지 않았다.
- **원인**: 하나의 patch 블록에서 같은 경로를 삭제한 뒤 다시 추가하는 두 연산을
  요청했다.
- **해결**: 삭제와 새 파일 추가를 별도 patch 호출로 분리했다. 첫 호출은 원자적으로
  실패해 부분 변경은 없었다.

### 이슈 2: zsh에서 `status`를 검증용 변수로 사용할 수 없음

- **증상**: doctor 첫 실행 뒤 `zsh: read-only variable: status`로 후속 검사 명령이
  중단됐다.
- **원인**: zsh가 `status`를 예약된 읽기 전용 변수로 제공한다.
- **해결**: 검증 변수명을 `doctor_rc`로 바꾸고 doctor와 후속 검사를 다시 실행했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml")'`
- [x] 구성된 저장소와 구성 없는 임시 루트에서 `scripts/doctor` 실행

## 결과

Claude Code와 Codex가 동일한 최신 저장소 계약을 자동으로 읽고, 작업 전에
`scripts/doctor` 하나로 로컬 환경을 안전하게 판별할 수 있다. setup은 상태 탐색과
분리되어 기존 clean·빌드·에디터 재설치 부작용을 명시적으로 요청받은 경우에만 수행한다.
