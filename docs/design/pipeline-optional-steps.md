# 설계 결정: 파이프라인 optional 스텝 `|> ?.`

- **상태**: 설계 확정, 미구현 — 대상 릴리스 0.4
- **RFC**: [Discussion #64](https://github.com/load28/tt/discussions/64)
  (숙의 4라운드 + 방출 형태 3라운드, 진행자 합의 2건)
- **태스크**: [TASK-245](../tasks/TASK-245-pipeline-optional-step-rfc.md) (RFC 정리)
- **전제 설계**: [`pipeline-operator.md`](./pipeline-operator.md) — 파이프라인
  연산자의 문법·방출·구조 파싱 계약. 이 문서는 그 §3.2가 "1차 범위에서 제외"로
  남겨 둔 `?.` 시작 스텝 하나만 확정한다.

이 문서는 Discussion #64의 숙의 결과를 규범 형태로 정리한 것이다. 라운드별 발언은
§8에 경과로만 남기고, §2~§7이 구현이 따라야 하는 계약이다.

## 1. 문제

파이프라인의 포스트픽스 스텝은 `.`으로 시작한다 (`pipeline-operator.md` §3.2):

```tt
const words = input |> .trim() |> .split(",");
```

파이프 값이 nullish일 수 있으면 이 형태를 쓸 수 없다. 지금은 스텝마다 화살표를
씌워 직접 보호해야 한다:

```tt
const name = user |> (u => u?.profile) |> (p => p?.displayName);
```

`.` 스텝이 흡수하려던 비용(래퍼 화살표)이 nullish 경로에서 그대로 되돌아온다.

## 2. 검토한 선택지

| | 형태 | 평가 |
|---|---|---|
| (A) optional 포스트픽스 스텝 (선택) | `user \|> ?.profile` | `?.`은 유효 TS의 어떤 스텝 시작 위치에도 올 수 없어 통과 계약이 안전하다. 의미는 JS optional chaining과 동일해 새로 배울 규칙이 없고, 방출은 기존 포스트픽스 경로에 `?.`을 잇는 것으로 끝난다 |
| (B) 현행 유지 | `user \|> (u => u?.profile)` | 새 문법이 없다는 장점은 있으나, nullish 경로에서 파이프라인의 존재 이유인 래퍼 제거가 무효가 된다. 스텝마다 반복되는 래퍼는 체인이 길어질수록 비용이 커진다 |
| (C) null-aware 파이프 연산자 | `user ?\|> f` (nullish면 스텝 스킵) | 파이프에 **새 제어 흐름**을 도입한다. 스킵 여부가 연산자에 숨어 값 흐름을 읽기 어렵고, `undefined`를 정상 입력으로 다루는 스텝과 의미가 충돌한다. 순수 TS 방출도 조건 분기 없이는 불가능하다 |

**(A)를 채택했다.** 결정 근거는 "tt은 TypeScript에 없는 의미를 만들지 않는다"는
쪽으로 수렴했다 — `value |> ?.member`는 `(value)?.member`와 **정의상 같다**.

## 3. 확정 문법

optional 스텝은 `?.`로 시작하고, 한 스텝 안에서 JS optional chain의 전체
포스트픽스 tail을 이어 붙일 수 있다.

```
optional-step    ::= optional-start postfix-segment*
optional-start   ::= "?." 이름 | "?." "[" 식 "]" | "?." "(" 인자들 ")"
postfix-segment  ::= "." 이름       | "?." 이름
                   | "[" 식 "]"     | "?." "[" 식 "]"
                   | "(" 인자들 ")" | "?." "(" 인자들 ")"
```

- 스텝 경계는 기존 규칙과 같다 — 다음 최상위 `|>` 또는 식 종결자(`;` `,` `)`
  `]` `}`)까지.
- 단독 `?.(...)` (optional call)도 포함한다.
- 첫 유의 바이트 두 개가 `?.`이면 optional 스텝, `.`이면 일반 포스트픽스 스텝,
  그 외는 적용 스텝이다. 판별에 식 수준 파싱이 필요 없다는 성질은 유지된다.

**tail을 제한하지 않기로 한 근거**: 초안에서는 0.4 범위를 `?.name`,
`?.name(args)`, `?.[expr]` 세 형태로 좁히자는 제안이 있었다. 그러나 기존 `.`
스텝이 이미 전체 포스트픽스 tail을 한 스텝으로 다루므로, optional 스텝만 좁히면
구현이 단순해지는 것이 아니라 **tt 전용 예외와 별도 검증기가 하나 늘어난다.**
파서는 구조 경계로 tail 전체를 수집하고 있고, 제한은 그 위에 추가 판정을 얹는
일이다. 그래서 제한 제안은 숙의 2라운드에서 세 관점 모두 철회했다.

## 4. 방출 계약

optional 스텝은 파이프 수신자 `E`에 tail을 그대로 잇는다. 런타임 헬퍼를 쓰지
않는다.

```
E |> ?.tail   →   (E)?.tail
```

괄호는 기존 포스트픽스 스텝과 **같은 판정기**(`codegen/core.rs`의
`push_receiver`)를 공유한다. `push_receiver`는 수신자 텍스트가 확정된 하나의
primary expression일 때만 괄호를 생략하고, 그렇지 않거나 아직 텍스트가 확정되지
않은 lowering 결과면 괄호를 유지한다.

| 입력 | 방출 |
|---|---|
| `value \|> ?.p` | `value?.p` |
| `make() \|> ?.p` | `make()?.p` |
| `values \|> ?.[index]` | `values?.[index]` |
| `handler \|> ?.(arg)` | `handler?.(arg)` |
| `await task \|> ?.p` | `(await task)?.p` |
| `a + b \|> ?.p` | `(a + b)?.p` |
| `user \|> ?.profile \|> ?.displayName` | `user?.profile?.displayName` |
| `user \|> ?.profile.name?.trim()` | `user?.profile.name?.trim()` |

- **optional 스텝 전용 긍정 목록이나 새 precedence 판정기는 만들지 않는다.**
  안전성이 같을 때 `value?.member`가 `(value)?.member`보다 읽기 쉽고 디버깅하기
  쉽다는 점이 공유 판정기를 택한 이유다. 보수적 판정이므로 괄호가 필요한
  경우를 놓치지 않는다.
- 괄호 필요성 판정과 **수신자 유효성 검사는 별개 문제**다. 후자는 §6.

## 5. 의미 계약

1. **JS optional chaining과 동일하다.** 짧은 순환(short-circuit) 범위, 평가
   순서, `?.()`의 `this` 바인딩 모두 방출된 TS가 그대로 결정한다. ttc가 추가
   의미를 만들지 않는다.
2. **파이프 입력은 한 번만 평가된다.** 수신자를 두 번 방출하는 형태
   (`E == null ? undefined : E.p`)는 쓰지 않는다.
3. **optional 스텝 뒤의 적용 스텝은 스킵되지 않는다.** `user |> ?.profile |>
   formatProfile`은 `formatProfile(user?.profile)`이고, `user`가 nullish면
   `formatProfile`은 `undefined`를 인자로 **실행된다.** 자동 스킵은 선택지 (C)와
   함께 기각됐다.
4. **타입은 TypeScript 소관이다.** `undefined`를 받는 스텝의 타입 오류는
   TypeScript가 사용자 텍스트 위에서 보고한다 — 에러 계층 분리 계약 유지.

## 6. 진단

- **원자적 인식**: `?.` 뒤의 tail이 §3 문법으로 완전히 인식되지 않으면 스텝을
  클레임하지 않고 tt 진단으로 보고한다. **부분 변환은 금지한다** — 절반만 변환된
  출력이나 verbatim 통과는 유효하지 않은 TS를 만들고, 이는
  `pipeline-operator.md` §5.1의 스트레이 `|>` 처리와 같은 계약 위반이다.
- **수신자 유효성**: 구문적으로 optional chain의 수신자가 될 수 없는 head
  (예: 벌거벗은 `super`)는 codegen 전에 거부한다. 괄호 생략 판정에 섞지 않고
  독립 검사로 둔다.
- 빈 tail(`x |> ?.`), `?.` 뒤 비식별자 등은 위 원자적 인식으로 수렴한다.

## 7. 완료 조건 — 검증 행렬

구현 태스크는 아래 다섯 항목을 **일반 포스트픽스 스텝과 optional 스텝 양쪽에**
적용해 통과시켜야 한다. 방출 형태를 공유하기로 한 결정의 대가가 이 행렬이다.

| 항목 | 확인 내용 |
|---|---|
| TypeScript 파싱 | 방출된 TS가 `--strict --noEmit`으로 파싱·검사된다 |
| 런타임 값 | node 실행 결과가 손으로 쓴 `(E)?.tail`과 같다 |
| 단일 평가 | 부수 효과가 있는 head가 정확히 한 번 평가된다 |
| optional call의 `this` | `obj \|> ?.m()` 형태에서 `this` 바인딩이 유지된다 |
| source map 동등성 | 스텝 텍스트의 매핑이 일반 포스트픽스 스텝과 같은 기준을 만족한다 |

## 8. 숙의 경과 (Discussion #64)

세 관점(Language Designer / Implementation Skeptic / User Advocate)이 두 논점을
차례로 다뤘고, 진행자가 각각 합의를 확정했다.

**논점 1 — 문법 범위 (4라운드)**

| 라운드 | 움직임 |
|---|---|
| 1 | 세 관점 모두 (A) 지지. Skeptic·User Advocate는 0.4 범위를 세 형태로 제한하자고 제안 |
| 2 | 기존 `.` 스텝이 이미 전체 tail을 한 스텝으로 처리한다는 사실이 확인되며 **제한 제안 철회** — 제한은 단순화가 아니라 별도 검증기 추가 |
| 3~4 | 전체 tail + 단독 `?.()` + 원자적 진단으로 입장 유지, 새 반례 없음 |
| 합의 1 | 선택지 (A), 전체 포스트픽스 tail, 단독 `?.()`, 미지원 형태는 원자적 tt 진단 |

**논점 2 — 괄호 방출 정책 (3라운드)**

| 라운드 | 움직임 |
|---|---|
| 1~2 | 항상 `(E)?.tail`로 감쌀지, 생략 조건을 둘지 대립 |
| 3 | 기존 `push_receiver`가 보수적 primary 판정을 이미 제공한다는 확인으로 수렴. Skeptic은 §7 검증 행렬 완수를 조건으로 수락 |
| 합의 2 | 공유 `push_receiver`, optional 전용 판정기 없음, 검증 행렬은 필수 완료 조건 |

## 9. 후속 작업

이 문서는 설계만 확정한다. 구현 태스크에서 함께 처리할 항목:

- `parser/pipes.rs`의 스텝 판별에 `?.` 시작 추가, AST/HIR의 스텝 종류 확장,
  `codegen/pipes.rs`의 tail 방출 (`push_receiver` 공유).
- sema: 원자적 인식 실패 진단과 수신자 유효성 검사, `errors.md` 메시지 추가.
- `flow` 합성의 첫 스텝은 포스트픽스 스텝을 금지한다
  (`pipeline-operator.md` §11.2). optional 스텝도 같은 금지에 포함되는지
  구현 태스크에서 확정하고 진단을 맞춘다.
- 테스트: `tests/compile.rs` 방출 계약, `tests/fixtures/` 스냅샷,
  `tests/passthrough.rs`의 `?.` 포함 유효 TS 바이트 보존,
  통합 테스트로 §7 행렬.
- 문서: 언어 표면이 바뀌므로 `docs/ai/tt.md`의 "No `?.`-starting step" 항목과
  README 예제를 구현과 같은 태스크에서 갱신한다.
