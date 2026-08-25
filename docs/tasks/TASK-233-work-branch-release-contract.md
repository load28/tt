# TASK-233: 작업 브랜치 기반 릴리스 계약

- **상태**: 완료
- **시작일**: 2026-08-26
- **완료일**: 2026-08-26
- **커밋**: —

## 목적

개발 변경을 main에 직접 넣지 않고 작업 브랜치에서 Dev 검증과 Production 승격까지
이어가는 계약을 `AGENTS.md`와 GitHub Actions에 일치시킨다.

## 범위

- 포함: 작업·릴리스 브랜치 규칙, Dev 원본 ref 입력, 실패한 Dev 브랜치 갱신과 재검증
- 제외: 버전 계산 규칙과 Production 자동 게시 조건 변경

## 의사결정

### 결정 1: Dev 준비가 명시적인 작업 ref를 받는다

- **상황**: 기존 Dev 준비는 항상 main에서 분기하므로 Production 승인 전까지 작업을 main 밖에 두는 흐름을 표현할 수 없었다.
- **검토한 대안**: main 고정 / 액션 실행 ref 재사용 / 별도 `source_ref` 입력.
- **선택과 근거**: 워크플로 정의는 main의 승인된 버전을 사용하면서 릴리스 대상만 독립적으로 고를 수 있도록 필수 `source_ref`를 받는다.

### 결정 2: 실패 수정은 원본 작업 브랜치에서 받아 기존 릴리스 브랜치에 병합한다

- **상황**: 실패 수정 커밋을 main과 릴리스 브랜치에 각각 cherry-pick하면 내용이 같아도 Production 계보가 끊어진다.
- **검토한 대안**: 릴리스 브랜치 직접 수정 / 매번 새 Dev 번호 할당 / 원본 작업 SHA 병합 후 같은 Dev 재검증.
- **선택과 근거**: 작업 브랜치를 단일 변경 원천으로 유지하고, 게시 전 실패에는 같은 Dev 릴리스 브랜치를 갱신해 모든 게이트와 산출물을 다시 만든다.

## 작업 내역

- 2026-08-26: `codex-task-233-release-source-workflow` 작업 브랜치를 main에서 만들었다.
- 2026-08-26: 기존 `0.3.0-dev.8`의 cherry-pick 계보가 Production 조상 검사를 통과하지 못하는 원인을 확인했다.
- 2026-08-26: Dev 준비 액션에 필수 `source_ref` 입력과 실패 재실행 시 원본 작업 SHA 병합을 추가했다.
- 2026-08-26: `AGENTS.md`에 작업 브랜치, Dev 준비·승인, Production PR·자동 게시와 실패 재시도 계약을 기록했다.

## 이슈 및 해결

### 이슈 1: 로컬 `codex` 브랜치가 `codex/` 네임스페이스를 막음

- **증상**: `codex/task-233-release-source-workflow` ref를 만들 수 없었다.
- **원인**: 이미 `refs/heads/codex`가 파일형 ref로 존재했다.
- **해결**: 기존 브랜치를 변경하지 않고 `codex-task-233-release-source-workflow`를 사용했다.

## 검증

- [x] `node --test npm/scripts/*.test.mjs packages/create-tt/test/*.test.mjs`
- [x] `./scripts/ci`

## 결과

개발 변경은 작업 브랜치에서 시작하며 Dev 준비가 그 브랜치의 정확한 SHA를 사용한다.
실패 수정도 작업 브랜치를 통해 기존 릴리스 브랜치에 반영되고, Production PR 병합 때만
main에 들어가는 계약과 실행 경로가 일치한다.
