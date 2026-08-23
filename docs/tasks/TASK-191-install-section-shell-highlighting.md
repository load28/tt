# TASK-191: 설치 페이지 셸 명령어 하이라이팅

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-24
- **커밋**: 이 변경을 포함하는 커밋

## 목적

공식 홈페이지 설치 페이지의 상세 절차에 있는 셸 명령어가 일반 텍스트로
렌더링된다. 상단 예제와 같은 빌드 시점 구문 강조를 상세 명령어에도 적용한다.

## 범위

- 포함: 설치 상세 섹션 코드의 Shiki 하이라이팅 생성과 렌더링, 회귀 검증.
- 제외: 설치 문구와 명령 자체 변경, 다른 언어 문법 또는 홈페이지 디자인 변경.

## 의사결정

### 결정 1: 상세 섹션도 기존 빌드 시점 Shiki 파이프라인에서 처리

- **상황**: 상단 예제는 `highlighted.json`의 생성된 HTML을 사용하지만 상세
  섹션은 `section.code` 원문을 React 텍스트로 렌더링하고 있었다.
- **검토한 대안**: 브라우저에서 Shiki 실행 / CSS로 명령어를 추정해 색칠 /
  기존 생성 스크립트에서 섹션별 HTML 생성.
- **선택과 근거**: 기존 `scripts/highlight.ts`가 섹션 배열도 함께 처리하도록 했다.
  클라이언트 번들에 하이라이터를 추가하지 않고, 상단 코드와 같은 문법·테마를
  단일 경로에서 적용할 수 있다.

### 결정 2: 섹션 하이라이트를 별도 생성 파일로 유지

- **상황**: 기존 `highlighted.json`은 토픽 ID에서 상단 코드 HTML 문자열로 가는
  공개 형태이며 상세 섹션은 토픽별 배열이 필요하다.
- **검토한 대안**: 기존 JSON을 중첩 객체로 변경 / 별도
  `highlighted-sections.json` 생성.
- **선택과 근거**: 별도 파일을 생성했다. 기존 상단 코드 소비 계약을 바꾸지 않으며,
  섹션이 있는 토픽만 담는 `Partial<Record<TopicId, string[]>>`로 실제 데이터 형태를
  표현한다. 렌더 시 누락은 명시적 오류로 처리해 빈 코드 블록을 만들지 않는다.

## 작업 내역

- 2026-08-23: `website/scripts/highlight.ts`와 `ReferencePage.tsx`의 코드 블록
  경로를 조사해 상단 예제만 생성된 HTML을 사용하고 상세 섹션은 원문을 직접
  렌더링하는 차이를 확인했다.
- 2026-08-24: 토픽의 언어 선택을 `topicLanguage`로 공통화하고, 섹션 코드를 같은
  Shiki `shellscript` 문법과 `github-dark-default` 테마로 변환해
  `highlighted-sections.json`에 생성하도록 했다.
- 2026-08-24: `GuideSections`가 생성된 HTML을 렌더링하도록 바꾸고, 생성 결과가
  없거나 섹션 수와 맞지 않으면 빌드 중 명시적으로 실패하도록 했다.
- 2026-08-24: 타입 검사, 프로덕션 빌드와 33개 경로 prerender, 생성된 `/install`
  HTML의 명령별 색상 span, Rust 검증 게이트를 확인했다.

## 이슈 및 해결

### 이슈 1: 샌드박스에서 prerender preview 서버 포트 바인딩 실패

- **증상**: 첫 `bun run build`가 `listen EPERM: operation not permitted ::1`로
  클라이언트·서버 번들 생성 뒤 중단됐다.
- **원인**: TanStack Start prerender가 로컬 preview 서버를 열지만 샌드박스가
  IPv6 loopback 포트 바인딩을 제한했다.
- **해결**: 같은 빌드를 승인된 포트 권한 환경에서 다시 실행했고 33개 경로가 모두
  prerender되는 것을 확인했다. 코드 변경이나 우회는 필요하지 않았다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `website`: `bun run typecheck`
- [x] `website`: `bun run build` (33개 경로 prerender)

## 결과

`website/scripts/highlight.ts`가 설치 상세 섹션용 HTML도 생성하고,
`website/src/ui/ReferencePage.tsx`가 이를 렌더링한다. 공식 홈페이지의 영문·한글
설치 페이지에서 상세 셸 명령어와 주석, 옵션, 환경 변수가 구문 강조된다.
