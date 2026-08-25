# TASK-227: 바인딩 이름 `match`가 tt match로 오인된다

- **상태**: 대기
- **시작일**: —
- **완료일**: —
- **커밋**: —

## 목적

TASK-223의 코퍼스 차등 테스트가 **이 저장소 자신의 파일**에서 계약 1 위반을
찾았다. `website/scripts/essay.ts`는 유효한 TypeScript인데 ttc가 거부한다.

최소 재현:

```ts
declare const xs: string[];
for (const match of xs) {
  console.log(match);
}
```

```
error[malformed-match]: tt `match` could not be parsed
 --> min.ts:2:12
  |
2 | for (const match of xs) {
  |            ^^^^^
```

`match`는 여기서 **바인딩 이름**이다. TypeScript에서 `match`는 예약어가 아니므로
변수·매개변수·프로퍼티 이름으로 자유롭게 쓰인다. `tests/passthrough.rs`가
메서드 이름(`class Router { match(...) }`), 프로퍼티(`s?.match(re)`),
`String.prototype.match`는 고정하고 있지만 **바인딩 이름**은 빠져 있었다.

## 범위

- 포함:
  - `src/parser/matches.rs`의 `committed` 판정 수정. 현재 식별자 갈래는
    `match` 다음에 식별자가 오기만 하면 tt match라고 확신한다 —
    `match of`, `match satisfies T`, `match as X`가 모두 걸린다.
  - 책임 있는 계층에서 일반화: 특정 문자열(`of`)을 제외하는 것이 아니라,
    **`match`가 식이 시작될 수 있는 자리인가**와 **뒤따르는 블록이 arm을
    갖는가**로 판정한다.
  - `tests/passthrough.rs`에 바인딩 이름 계열 회귀 케이스.
- 제외: 코퍼스 기계 자체 (TASK-223).

## 의사결정

## 작업 내역

## 이슈 및 해결

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test` — `--test corpus` 포함

## 결과
