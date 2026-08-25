# TASK-227: 바인딩 이름 `match`가 tt match로 오인된다

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: (아래 "작업 내역")

## 목적

TASK-223의 코퍼스 차등 테스트가 **이 저장소 자신의 파일**에서 계약 1 위반을
찾았다. `website/scripts/essay.ts`는 유효한 TypeScript인데 ttc가 거부한다.

```ts
declare const xs: string[];
for (const match of xs) {
  console.log(match);
}
```

```
error[malformed-match]: tt `match` could not be parsed
```

## 범위

- 포함:
  - `src/parser/matches.rs`의 `committed` 판정 수정
  - 책임 있는 계층에서 일반화 — 특정 문자열을 제외하는 것이 아니라 구조로 판정
  - `tests/passthrough.rs`에 회귀 케이스
- 제외: 코퍼스 기계 자체 (TASK-223)

## 의사결정

### 1. 조사해 보니 훨씬 넓은 구멍이었다

처음 보고된 것은 `for…of` 바인딩 하나였지만, 재현을 넓혀 보니 같은 원인의
다른 얼굴이 계속 나왔다.

| 유효한 TypeScript | 결과 (수정 전) |
|---|---|
| `for (const match of xs) { console.log(match); }` | 거부 |
| `function match(x: number) { return f(y => y + x); }` | 거부 |
| `class C { match(x: number) { return f(y => y + x); } }` | 거부 |
| `const o = { match(x: number) { return f(y => y + x); } };` | 거부 |

기존 `tests/passthrough.rs`의 `class_method_named_match`가 통과하고 있던 이유가
드러났다 — 그 케이스에는 **반환 타입 주석이 있었다**(`match(p: string): boolean`).
`)`와 `{` 사이에 `: boolean`이 끼어 있어서 판정이 발동하지 않았을 뿐이다.
주석을 지우면 그 테스트도 실패한다. 표본이 우연히 통과하고 있었다.

### 2. 판정은 **본문이 무엇인가**로 한다

`match`는 예약어가 아니므로 TypeScript에서 평범한 이름이다. 메서드, 바인딩,
함수 — 전부 `match`, 무언가, 그리고 블록이다. match 식과 **실루엣이 같다**.
같지 않은 것은 블록의 내용이고, 답은 거기서 나와야 한다.

arm 목록은 `<pattern> => <body>`의 쉼표 목록이다. 그래서 본문의 **자기 중괄호
레벨**에서:

- 첫 토큰이 문장 키워드(`return`, `const`, …)일 수 없다. 패턴은 태그·리터럴·
  `_`·튜플이다. `true`/`false`는 예약어이면서 동시에 리터럴 패턴이므로 남긴다.
- `;`보다 먼저 `=>`가 나와야 한다. 문장 목록은 세미콜론으로 나뉘고, arm 목록은
  자기 레벨에서 세미콜론에 도달하지 않는다.

**깊이가 핵심이다.** `return f(y => y)`의 화살표는 블록이 아니라 호출에 속한다.
"본문 어딘가에 화살표가 있으면"이라는 기존 판정이 반환 타입 주석 없는 메서드를
전부 붙잡은 이유가 정확히 이것이다.

### 3. 위치 기반 가드는 넣지 않았다

"`const` 뒤의 식별자는 선언된 이름"도 참이고 값싸지만, 결정 2의 규칙이 이미
모든 사례를 덮는다. 겹치는 두 번째 규칙은 커버리지를 늘리지 않으면서 읽을 것만
늘린다. 코퍼스 테스트가 이 판단의 검사 장치다.

### 4. 남는 모호함은 남는다 — 그리고 그것은 tt 쪽이 옳다

`class C { match(x) { y => y } }`는 유효한 TypeScript(쓸모없는 화살표 식
문장)이면서 동시에 유효한 tt match다. 텍스트 자체가 두 가지 뜻을 갖는다.
`.tt` 파일에서 tt로 읽는 것이 옳고, `.ts`에서는 통과해야 하지만 — 그런 코드는
병적이라 규칙을 더 복잡하게 만들 값어치가 없다. 판단을 여기 남긴다.

## 작업 내역

1. `src/parser/matches.rs`: `body_reads_as_arms(src, tokens, open)` 추가.
   식별자 갈래와 괄호 갈래가 **둘 다** 이것으로 판정한다 — 두 갈래가 같은
   구멍을 갖고 있었으므로 답도 하나여야 한다.
2. `tests/passthrough.rs`: 회귀 6건 — `for…of` 바인딩(화살표 있는 본문 포함),
   주석 없는 함수·클래스 메서드·객체 리터럴 메서드, 문장 키워드로 시작하는 본문.

## 이슈 및 해결

- **증상**: 처음에는 `for…of` 한 건으로 보였다.
- **원인**: 재현을 넓히지 않았다면 위치 기반(`const` 뒤) 가드로 "고쳤을" 것이고,
  메서드 세 갈래는 그대로 남았을 것이다.
- **해결**: 같은 실루엣을 갖는 입력을 전부 만들어 본 뒤에 판정을 설계했다.
  구조적으로 같은 입력에 같은 답이 나온다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 전체 초록
- [x] `TTC_CORPUS_FULL=1 cargo test --test corpus` — 594개 중 534개 통과,
      60개 유효한 TypeScript 아님, **위반 0**

## 결과

코퍼스가 찾은 위반이 사라졌고, 진짜 near-miss는 그대로 보고된다:

```
const r = match value { A(n) => n, B => 0 };
→ error[malformed-match]: tt `match` could not be parsed
  = help: a match scrutinee is parenthesized — `match (<expression>) { ... }`: `(value)`
```

기계가 만들어진 날 기계가 찾은 것을 고쳤다. 손으로 쓴 표본이 6년을 봐도 못 볼
종류의 구멍이었다 — 표본 자신이 반환 타입 주석 때문에 우연히 통과하고 있었기
때문이다.

### 변경 파일

- `src/parser/matches.rs`
- `tests/passthrough.rs`
- `docs/tasks/INDEX.md`
