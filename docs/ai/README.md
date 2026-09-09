# AI와 함께 tt 쓰기

[`tt.md`](./tt.md)는 **AI 코딩 도구에게 tt을 가르치는 컨텍스트 문서**입니다.
This single guide contains everything an AI needs to read and write `.tt`
correctly: the complete language surface, including literal and `is` match
patterns, passthrough rules, common mistakes, and build setup. It is written in
English and is self-contained, so it works outside this repository.
사람이 읽는 문서가 아니므로 컨텍스트 비용을 줄이기 위해 최대한 압축된
표기(짧은 규칙 나열)를 씁니다. 사람을 위한 시작 안내는 [README](../../README.ko.md)에 있습니다.

## 사용법

tt을 쓰는 **여러분의 프로젝트**에 `tt.md`를 복사해 넣고, 사용하는 AI 도구가
읽도록 연결하면 됩니다.

```sh
mkdir -p docs
curl -o docs/tt-ai-guide.md \
  https://raw.githubusercontent.com/load28/tt/main/docs/ai/tt.md
```

| 도구 | 연결 방법 |
|------|-----------|
| Claude Code | 프로젝트 `CLAUDE.md`에 `@docs/tt-ai-guide.md` 한 줄을 추가하거나, 내용을 `CLAUDE.md`에 직접 붙여넣기 |
| Cursor | `.cursor/rules/tt.mdc`로 저장 (프론트매터에 `globs: ["**/*.tt"]`을 주면 `.tt` 파일 작업 시 자동 적용) |
| GitHub Copilot | `.github/copilot-instructions.md`에 내용 추가 |
| Codex · Jules 등 AGENTS.md 계열 | 프로젝트 `AGENTS.md`에 내용 추가 또는 링크 |
| 그 외 (챗 기반) | 대화 시작 시 파일 내용을 시스템 프롬프트/첫 메시지로 붙여넣기 |

`npm install @openload28/tt-lang`을 쓰는 프로젝트라면 설치된 패키지 대신 위처럼 저장소의
최신 문서를 받아 두고, tt 버전을 올릴 때 함께 갱신하는 것을 권장합니다.

**같은 가이드가 컴파일러에도 임베드되어 있습니다** — 설치된 ttc에서
`npx ttc help`(주제 목록), `npx ttc help match`(주제별),
`npx ttc help all`(전체)로 꺼낼 수 있어, AI가 작업 중 네트워크 없이 문법을
찾아보거나 컨텍스트 파일 없이도 스스로 가이드를 조회할 수 있습니다.

## 이 문서가 다루는 것

- **통과 계약**: 유효한 TS는 그대로 유효한 `.tt`이고, 어긋난 tt 구문은
  에러가 아니라 조용히 통과된다는 것 — AI가 가장 자주 빠지는 함정.
- **Language rules**: required parentheses and semicolons, name-based
  bindings, literal and `is` pattern families, and coverage behavior for
  guarded and nested arms.
- **`@tt/std` 치트시트**: `Option`/`Result`의 필드명(`value`/`error`)과
  콤비네이터, 파이프라인용 `*P` 변형.
- **Build pipeline**: `ttc` commands, the TypeScript content mapper,
  `tsc --runExternalCode`, and bundler integration through
  `@openload28/unplugin-tt`.
- **에러 읽는 법과 마무리 체크리스트**.

## 유지 관리

이 문서는 컴파일러에 그대로 포함되는 AI용 언어 가이드입니다. **언어 표면이
바뀌면 이 문서도 함께 갱신해야 합니다.** 구현과 어긋나면 버그로 취급합니다.
