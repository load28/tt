# TASK-217: VS Code 확장 completion 테스트의 불안정성 조사

- **상태**: 대기
- **시작일**: —
- **완료일**: —
- **커밋**: —

## 목적

`editors/vscode/server/src/test/completion.test.ts`의 probe/멤버 완성 케이스가
로컬 컨테이너에서 결정적으로 실패하는데, **어떤 케이스가 실패하는지가 실행
환경에 따라 달라진다**. TASK-215 작업 중 확인한 내용:

| 확장 빌드 | `ttc` 바이너리 | 실패 케이스 |
|---|---|---|
| 이 브랜치 | 이 브랜치 | `a probe carries the pipeline's type through earlier steps` |
| 이 브랜치 | `origin/main` | `a match arm binding's members come from the emit` |
| `origin/main` | 이 브랜치 | 위 두 개 |
| `origin/main` | `origin/main` | `a pipeline step's members need a probe`, `a match arm binding's...` |

`/tmp`에 쌓인 이전 워크스페이스 1151개를 지우고 `--test-concurrency=1`로 직렬
실행해도 같다. 즉 실패 집합이 어느 한 변수의 함수가 아니다. 확장의 다른 스위트
(`server`, `typedcheck`, `engine`, `sidecar`)는 같은 조건에서 skip 0으로 전부
통과하므로, 문제는 이 파일에 국한된다.

이 상태로는 이 스위트가 회귀를 잡는지 아무도 신뢰할 수 없다. CI는 초록이므로
환경 민감성일 가능성이 높지만, "CI에서는 되니까"는 품질 보장이 아니다.

## 범위

- 포함:
  - 실패의 실제 원인 특정 — 한 파일의 테스트들이 **엔진 서버 세션 하나를
    공유**하는 구조(`after(() => engine.shutdownEngineServer())`)가 첫 용의자다.
    프로젝트 루트 결정이 임시 디렉터리 위치에 따라 달라지는지, 이전 케이스가
    남긴 프로젝트 캐시가 다음 케이스의 답을 바꾸는지 확인한다.
  - 원인이 테스트 격리라면 케이스마다 세션을 분리하거나 워크스페이스를
    명시적으로 격리한다.
  - 원인이 컴파일러/엔진이라면 해당 계층에 회귀 테스트를 만들고 고친다.
- 제외: 실패를 `skip`으로 덮는 것. 스킵은 CI에서 초록으로 보이지만 아무것도
  보장하지 않는다(CI의 기존 "skip은 pass가 아니다" 가드와 같은 이유).

## 의사결정

## 작업 내역

## 이슈 및 해결

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## 결과
