# match `is` 패턴

**상태:** Accepted 표면. [Discussion #93](https://github.com/load28/tt/discussions/93)
**추적:** [#91](https://github.com/load28/tt/issues/91) — unknown 관용구. 추론 격차(G1/G5)와 한 줄에 두지 않는다.
**구현 문서:** 이 문서가 잠긴 뒤에만 연다. 코드는 다른 에이전트가 친다.

이 문서는 잠긴 방향을 구현 문서가 그대로 따를 계약으로 자른다. 새 방향을 열지 않는다.

## 1. 결론

호스트가 `unknown`을 주는 자리(`catch`, JSON, 이벤트)에서 클래스 계층을 `match`로 좁힌다.

- 소진성은 없다. `_` 암은 필수다.
- 런타임 의미는 JavaScript `instanceof`다.
- 좁힌 값은 `const`로 물질화한다.
- ttc는 타입을 모른다. 서브클래스 가리기를 진단하지 않는다.
- P5는 타입 추론 격차가 아니다.

대표 입력과 방출:

```tt
const msg = match (err) {
  is SyntaxError { message } => `bad syntax: ${message}`,
  is RangeError | is TypeError => "bad value",
  is Error { message } => message,
  _ => String(err),
};
```

```ts
const msg = ((() => {
  const $tt_m = err;
  if ($tt_m instanceof SyntaxError) {
    const { message } = $tt_m;
    return (`bad syntax: ${message}`);
  }
  if ($tt_m instanceof RangeError || $tt_m instanceof TypeError) {
    return ("bad value");
  }
  if ($tt_m instanceof Error) {
    const { message } = $tt_m;
    return (message);
  }
  return (String(err));
})());
```

기존 match의 IIFE + `$tt_m` 물질화 계약을 그대로 쓴다. 새 런타임 헬퍼를 만들지 않는다.

## 2. 문법

`is` 패턴은 **match 암만** 받는다. if let / let-else 자리는 열지 않는다.

```
is-pattern     = "is" type-path [ object-bind ]
or-is-pattern  = is-pattern { "|" is-pattern }
object-bind    = "{" bind-field { "," bind-field } [ "," ] "}"
bind-field     = ident [ ":" ident ]
type-path      = ident { "." ident }
```

받는 형태:

```tt
is Type
is Type { field }
is Type { field: rename, other }
is A | is B
```

`Type`은 `instanceof` 우변으로 그대로 나가는 생성자 경로다.

- 허용: `SyntaxError`, `ns.Foo`
- 거절: 제네릭 (`is Array<T>`), `typeof`, 호출 (`is Foo()`), 타입 연산자

`is A | is B`는 **타입 이름만**이다. or 다리에 `{ }`를 붙이지 않는다.

`is` 암은 기존 match 가드 `if expr`를 그대로 받는다. 새 가드 문법은 없다.

```tt
is SyntaxError { message } if message.length > 0 => message
```

바인딩에서 꺼낸 이름은 가드와 암 본문에서 보인다. 가드는 물질화 다음에 평가한다.

## 3. 의미

암은 **소스에 적힌 순서**대로 검사한다. 각 `is Type`은 `$tt_m instanceof Type`이다.

| 표면 | 런타임 |
|---|---|
| `is Type` | `if ($tt_m instanceof Type)` |
| `is Type { a, b: c }` | `instanceof` 성공 뒤 `const { a, b: c } = $tt_m` |
| `is A \| is B` | `if ($tt_m instanceof A \|\| $tt_m instanceof B)` |
| `is Type { … } if expr` | `instanceof`와 물질화 다음 `if (expr)` |
| `_` | 마지막 fallback. `is` match에 필수 |

바인딩 없는 `is Type`은 물질화 없이 `instanceof`만 낸다.
빈 `{ }`는 이 형태와 같지 않다. 거절한다.

`object-bind`는 JavaScript 객체 분해와 같은 이름 규칙이다. 필드가 클래스 프로퍼티가 아니면 tsc가 잡는다. ttc는 프로퍼티 존재를 검사하지 않는다.

rest 바인딩(`{ message, ...rest }`)과 중첩 `is`는 이 범위에 없다.

## 4. 생성자 정체성

생성자 정체성은 **Type 경로의 소스 텍스트**다. `SyntaxError`와 `ns.SyntaxError`는 다른 경로다. ttc는 둘이 같은 값인지 묻지 않는다.

`{ }` · rename · or 포장은 정체성을 바꾸지 않는다.

한 match 안에서 같은 경로가 **두 암의 정체성 집합에 모두 들어가면** `match-duplicate-arm`으로 거절한다.

```tt
// reject — 같은 SyntaxError
is SyntaxError => "a",
is SyntaxError { message } => message,

// reject — TypeError가 or와 단독에 둘 다 있음
is RangeError | is TypeError => "a",
is TypeError => "b",

// legal — 경로가 다름. 가리기는 진단하지 않음
is Error => "a",
is SyntaxError => "b",
```

or-암의 정체성 집합은 다리 Type 경로의 합집합이다.

## 5. 거절

진단은 기존 라벨 체계를 따른다. help는 한 줄이다.

| 입력 | 결과 | help |
|---|---|---|
| `is SyntaxError(message)` | 거절 | `{ message }`로 고친다 |
| `is SyntaxError { }` | 거절 | 중괄호를 뺀다 |
| `is A { x } \| is B` | 거절 | or-암은 타입 이름만. 값을 빼려면 암을 쪼갠다 |
| `is A { x } \| is B { x }` | 거절 | 위와 같음 |
| `is` 암이 있는 match에 `_` 없음 | 거절 | `_` 암을 넣는다 |
| 같은 Type 경로가 두 암 | `match-duplicate-arm` | 중복 암을 제거한다 |
| `is`와 태그·튜플·variant 한 match | 거절 | `is` match에는 `is`·리터럴·`_`만 둔다 |
| if let / let-else의 `is` | 이 범위에서 파싱하지 않음 | — |
| rest, 중첩 `is` | 거절 | — |
| 제네릭·`typeof` 우변 | 거절 | — |

```tt
// legal
is SyntaxError
is SyntaxError { message }
is SyntaxError { message: msg }
is RangeError | is TypeError

// reject
is SyntaxError(message)
is SyntaxError { }
is SyntaxError { message } | is TypeError
is SyntaxError { message } | is TypeError { message }
```

`()` 형태는 variant 생성자 페이로드처럼 보이지만 방출은 프로퍼티 분해다. 받아서 고치지 않는다. 거절하고 `{ }`만 가리킨다.

## 6. 혼용

`is`가 **하나라도** 있는 match의 암은 다음만 받는다.

- `is` 암 (`is Type`, `is Type { … }`, `is A | is B`, 가드 포함)
- 리터럴 암 (기존 리터럴 패턴)
- `_`

태그·튜플·variant 암은 거절한다. 리터럴과 `is`는 둘 다 `_`가 필요해서 소진성 모델을 섞지 않는다.

```tt
// legal
match (x) {
  is SyntaxError { message } => message,
  "ok" => "ok",
  _ => "other",
}

// reject
match (x) {
  is SyntaxError => "err",
  SomeTag => "tag",
  _ => "other",
}
```

`is`가 없는 match는 기존 규칙을 그대로 따른다. 이 문서가 그 규칙을 바꾸지 않는다.

## 7. 진단하지 않는 것

ttc는 클래스 계층을 모른다. 아래 문장을 언어 표면에 그대로 둔다.

> `is` 암은 적힌 순서대로 검사한다. 부모 클래스 암이 자식 클래스 암보다 앞에 있으면 자식 암은 도달하지 않는다. ttc는 이 가리기를 진단하지 않는다.

```tt
// 죽은 암. ttc는 침묵.
match (err) {
  is Error { message } => message,
  is SyntaxError { message } => `syntax: ${message}`,
  _ => String(err),
}
```

구조로 보는 건 같은 생성자 경로 반복뿐이다. 그건 계층 검사가 아니다.

TypeScript `catch` 변수 타입을 `unknown`에서 바꾸지 않는다. 좁히기는 `instanceof` 방출이 tsc에게 맡기는 것이다.

## 8. 방출 계약

기존 가드 match와 같은 if-체인 IIFE다.

1. scrutinee를 `$tt_m`으로 한 번 묶는다.
2. 암을 소스 순서대로 `if`로 나열한다.
3. `is Type` → `if ($tt_m instanceof Type)`.
4. `is A | is B` → `if ($tt_m instanceof A || $tt_m instanceof B)`. 다리 순서도 소스 순서다.
5. `{ }`가 있으면 `instanceof` 본문 첫 줄에 `const { … } = $tt_m`.
6. 가드가 있으면 물질화 다음 `if (expr)`. 가드 실패는 다음 암으로 떨어진다.
7. 리터럴 암은 기존 리터럴 방출을 쓴다.
8. `_`는 마지막 `return`.

가드 방출:

```tt
is SyntaxError { message } if message.length > 0 => message
```

```ts
if ($tt_m instanceof SyntaxError) {
  const { message } = $tt_m;
  if (message.length > 0) {
    return (message);
  }
}
```

or-암은 물질화가 없으므로 본문은 바로 `return`이다.

프로퍼티 분해가 던지면 그건 런타임 TypeScript/JS 동작이다. ttc가 catch하지 않는다.

## 9. 이번 범위에 없는 것

- 소진성 검사
- `if let` / `let-else`의 `is`
- rest 바인딩, 중첩 `is`
- 서브클래스 순서 진단
- 제네릭·`typeof` 우변
- `is`를 match 밖 타입 가드로 쓰기
- 태그드 유니언과 `is`를 한 match에 섞기

## 10. 구현 문서가 따를 완료 조건

구현 문서는 이 계약만 푼다. 파일·심볼·테스트 이름은 구현 문서가 정한다.

1. 위 문법의 legal 입력이 대표 방출과 같은 `instanceof` + `const` 물질화로 나간다.
2. 5절 표의 거절이 모두 컴파일 에러다. `()`는 `{ }` help, 빈 `{ }`는 중괄호를 빼라는 help.
3. `_` 없는 `is` match는 거절한다.
4. 같은 Type 경로 중복은 `match-duplicate-arm`이다. `is Error` 다음 `is SyntaxError`는 침묵이다.
5. `is` + 리터럴 + `_`는 받고, `is` + 태그는 거절한다.
6. 가드는 기존 `if expr`이며, 방출은 `instanceof` 다음 물질화 다음 `if`.
7. if let / let-else / rest / 중첩 `is` / 제네릭 우변은 이 범위의 완료가 아니다.

## 11. 참고

- [RFC: P5 is 패턴](https://github.com/load28/tt/discussions/93)
- [남은 타입 추론 격차](https://github.com/load28/tt/issues/91)
- [tt 1.0.0 로드맵](https://github.com/load28/tt/issues/90)
- [type-inference-gaps.md §7](./type-inference-gaps.md)
