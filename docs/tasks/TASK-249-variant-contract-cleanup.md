# TASK-249: 비태스크 파일 `variant` 계약 정합성

- **상태**: 진행 중
- **시작일**: 2026-08-27
- **완료일**: —
- **커밋**: —

## 목적

TASK-245 후에도 태스크 기록 밖의 현재 코드·메타데이터·사용자 가이드·
설계 문서에 남은 폐기된 tt `enum` 계약을 `variant`로 정리한다.

## 범위

- 포함: create-tt 시작 소스, 구조화 퍼저, 패키지 메타데이터, AI·현재
  설계 문서, 소스·테스트 주석과 현재 Unreleased 변경 기록
- 제외: `docs/tasks/` 과거 기록, TypeScript·Rust·JSON Schema가 소유하는
  `enum`, 삭제된 구 구현 이름을 역사적으로 인용하는 문장, 0.3 변경 기록

## 의사결정

### 결정 1: 문자열 치환이 아니라 소유권으로 `enum`을 분류

- **상황**: 저장소에는 tt의 구 선언, TypeScript 통과 테스트, Rust enum,
  JSON Schema `enum`, 과거 구현 이름이 함께 존재한다.
- **검토한 대안**: `enum`의 일괄 치환 / tt 태그드 유니언을 가리키는 현재
  계약만 전환.
- **선택과 근거**: TypeScript 통과 계약과 Rust 언어 구조를 보존해야 하므로,
  각 일치의 소유권과 문맥을 확인한 뒤 tt 계약만 `variant`로 바꾸다.

## 작업 내역

- 2026-08-27: 홈페이지·`docs/tasks/`를 제외한 저장소 전체의 `enum` 용례를
  소유권별로 조사했다.
- 2026-08-27: create-tt 시작 템플릿과 구조화 퍼저가 폐기된 tt `enum`
  문법을 생성하는 문제를 확인했다.
- 2026-08-27: 생성기·퍼저 출력을 `variant`로 바꾸고 create-tt 회귀 테스트를
  추가했다.
- 2026-08-27: 패키지 메타데이터, AI 가이드, Unreleased 변경 기록과 현재
  설계 문서의 용어를 `variant` 계약에 맞췄다.
- 2026-08-27: 컴파일러·편집기 내부의 현재 용어와 TextMate payload scope를
  정리하고 생성 문법을 갱신했다.

## 이슈 및 해결

### 이슈 1: 구조화 퍼저 확인에 crates.io 접근이 필요했다

- **증상**: 최초 `cargo check --manifest-path fuzz/Cargo.toml`이 샌드박스의
  DNS 제한으로 의존성을 받지 못했다.
- **원인**: fuzz 전용 lockfile의 crate가 로컬 캐시에 없었다.
- **해결**: 네트워크 권한으로 의존성을 받은 뒤 같은 명령이 통과함을 확인했다.

## 검증

- [x] `./scripts/ci`
- [x] `node --test packages/create-tt/test/installer.test.mjs`
- [x] `cargo check --manifest-path fuzz/Cargo.toml`
- [x] `npm --prefix editors/vscode run grammar:check`
- [x] 비태스크 파일의 폐기된 tt `enum` 잔여 검토

## 결과

create-tt와 구조화 퍼저가 `variant` 선언을 생성한다. 패키지 설명, AI·설계
문서, Unreleased 변경 기록, 컴파일러·편집기 용어와 문법 scope가 현재 언어
계약을 따른다. TypeScript·Rust·JSON Schema의 `enum`과 과거 태스크 기록은
보존했다. 전체 로컬 CI와 전용 검증이 통과했다.

변경 파일: `AGENTS.md`, `CHANGELOG.md`, `Cargo.toml`, `docs/ai/tt.md`,
`docs/design/*.md`, `editors/vscode/{server/src,syntaxes}/`,
`fuzz/fuzz_targets/generated_tt_compiles.rs`, `integrations/unplugin/README.md`,
`npm/tt-lang/package.json`, `packages/create-tt/`, `src/`, `tests/`,
`docs/tasks/INDEX.md`, 본 문서.
