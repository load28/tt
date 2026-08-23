# TASK-164: 웹사이트 `.rlx` React 가이드

- **상태**: 완료
- **시작일**: 2026-08-23
- **완료일**: 2026-08-23
- **커밋**: —

## 목적

웹사이트에서 `.rlx = TSX + rl` 계약과 React 사용법을 독립된 가이드로 보여준다.
방문자가 overview의 짧은 문구에 의존하지 않고 JSX 자식·속성에서 `match`를 쓰는
방법과 빌드 책임 경계를 확인할 수 있게 한다.

## 범위

- 포함: 영문·한글 `.rlx` 주제, TSX 코드 하이라이팅, 내비게이션과 코드 라벨.
- 제외: React 런타임 내장, 별도 데모 애플리케이션 임베드, 배포 설정 변경.

## 의사결정

### 결정 1: overview 문구가 아니라 독립 주제로 제공한다

- **상황**: `.rlx` 지원은 overview의 지원 범위 한 줄에만 있어 실제 문법과 설정을
  확인하기 어렵다.
- **검토한 대안**: overview 예제를 TSX로 바꾸면 기존 rl 입문 흐름이 사라진다.
  독립 주제는 `.rl` 흐름을 유지하면서 직접 링크할 수 있다.
- **선택과 근거**: Language 그룹에 `React / .rlx` 주제를 추가한다. 예제와 지원
  계약을 기존 reference page 구조 안에서 동일하게 제공한다.

### 결정 2: `.rlx` 예제는 rlx TextMate 문법으로 하이라이팅한다

- **상황**: 현재 사이트 생성기는 모든 언어 예제를 `source.rl` 문법으로 처리한다.
- **검토한 대안**: TS 문법으로 처리하면 JSX는 보이지만 rl 구문 scope가 빠진다.
  문자열 후처리는 문법 소유권을 중복 구현한다.
- **선택과 근거**: VS Code 확장의 생성된 `source.rlx` 문법을 사이트 highlighter에
  함께 등록하고 topic 종류로 선택한다.

## 작업 내역

- 2026-08-23: TASK-164를 등록하고 웹사이트 content·highlight 생성 경계를 확인했다.
- 2026-08-23: Start 그룹에 영문·한글 `React / .rlx` 주제를 추가했다. JSX 자식과
  속성 expression container, concise arrow, `.rlx` import와 런타임 책임을 문서화했다.
- 2026-08-23: 사이트 highlighter가 생성된 rlx TextMate 문법을 직접 읽도록 연결하고
  코드 라벨, 브랜드 설명, sitemap과 highlighted 산출물을 갱신했다.
- 2026-08-23: `npm run typecheck`와 `npm run build`를 실행해 31개 페이지의
  prerender까지 통과했다.

## 이슈 및 해결

없음.

## 검증

- [x] `npm run typecheck`
- [x] `npm run build`

## 결과

웹사이트에 `/rlx`와 `/ko/rlx` 가이드를 추가했다. 두 페이지는 rlx 문법으로
하이라이팅된 React 예제와 TSX 통과·평가 순서·도구 체인·런타임 책임 계약을
독립적으로 설명한다.
