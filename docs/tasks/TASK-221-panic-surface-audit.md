# TASK-221: `unwrap`/`expect` 감사 — 안전망에서 사고 자체로

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: (아래 "작업 내역")

## 목적

TASK-214는 컴파일러가 패닉할 때 **어떻게 보고하고 살아남는지**를 정했다. 사고를
없애지는 않았다. `src/`의 현황(grep 기준): `unwrap()` 78, `expect(` 100,
`panic!`/`unreachable!` 77.

전부가 문제는 아니다. 문제는 **어느 것이 어느 쪽인지 구분이 문서화되어 있지
않다**는 점이다. 사용자 입력으로 도달 가능한 `unwrap`은 컴파일러 버그이고,
도달 불가능한 것은 그 이유가 적혀 있어야 한다.

## 범위

- 포함:
  - 세 부류로 분류: (a) 사용자 입력으로 도달 가능 — 진단이나 총함수로 고친다,
    (b) 상위 계층이 보장하므로 도달 불가 — **왜** 도달 불가한지 쓴다,
    (c) 이미 의도된 ICE — `ice.rs`의 명명된 invariant로 표현할 수 있는지 검토.
  - 우선순위는 사용자 입력에 가까운 계층부터.
- 제외: 일괄 치환.

## 의사결정

### 0. 목적의 숫자가 틀렸다 — 감사의 첫 결과

`78 / 100 / 77`은 `#[cfg(test)]` 모듈과 doc 예제를 포함한 grep이다. 테스트가
`unwrap`하는 것은 테스트가 실패하는 방법이지 컴파일러의 패닉 표면이 아니다.
제품 코드만 세면:

| | 감사 전 | 감사 후 |
|---|---|---|
| `unwrap()` | 9 | **0** |
| `expect(` | 26 | 25 |
| `panic!`/`unreachable!` | 61 | 11 (+ `ice::bug!` 53) |

`codegen/core.rs`가 45건으로 가장 많았는데, 전부 `panic!("internal compiler
error: ...")`였다 — 부류 (c)다.

### 1. (c) 검토 결과 — `Invariant`가 아니라 `ice::bug!`

`Invariant`는 **검증기의 어휘**다. 각 변형이 `program-lowering.md`의 한
문장이고, 네 개의 named validator가 만든다. codegen의 "이 모양이 방출기에
도달할 리 없다"는 다른 종류이고, 45개를 변형으로 밀어넣으면 그 설계가 희석된다.

그런데 검토하다 실제 결함이 나왔다. `ice::report()`가 이미
`error: internal compiler error: {message}`를 쓰는데, 사이트들이 메시지에 같은
접두사를 또 쓰고 있었다:

```
error: internal compiler error: internal compiler error: target node has no source span
```

**53개 사이트 전부.** `--server`의 오류 필드도 같은 이유로 이중이었다.

그래서 (c)의 답은 "이름을 붙인다"가 아니라 "**문구의 소유자를 한 곳으로
옮긴다"였다. `ice::bug!` 매크로가 그 자리다 — 호출자는 *무엇이* 깨졌는지만
쓰고, 접두사와 "이것은 ttc의 버그입니다" 일체는 리포터가 쓴다. 호출부에
`ice::bug!`라고 적혀 있다는 것 자체가 "이건 컴파일러 버그다"를 말한다.

접두사가 한 곳에만 있는지는 **크레이트 자신의 텍스트가 검사한다**
(`the_prefix_is_written_in_exactly_one_place`). 로워링 한 줄을 읽어서는 리뷰어가
잡을 수 없는 종류의 실수이므로, 사람이 아니라 테스트가 지키게 했다.

### 2. (a) 도달 가능한 것 — 총함수로

세 곳이 사용자 입력으로 도달 가능했다.

| 사이트 | 입력 | 처리 |
|---|---|---|
| `main.rs` `out_name.file_name().unwrap()` | `-o`와 입력 경로의 조합 | 파일 이름이 없으면 경로 전체를 쓴다 |
| `main.rs` `job.file.file_name().unwrap()` (배너) | 입력 경로 | 같음 |
| `main.rs` `.lock().expect("extern cache")` ×2 | 다른 잡의 패닉 | poison을 딛고 지나간다 |

마지막 것이 미묘하다. poisoned lock은 **다른 잡이 이미 패닉했다**는 뜻이고,
그 패닉이 보고되어야 할 실패다. 여기서 두 번째로 패닉하면 첫 번째를 묻는다.
맵은 삽입만 되고 반쯤 쓰인 상태로 남지 않으므로 내용은 건전하다 —
`into_inner()`로 지나가는 것이 옳다.

### 3. (b) 도달 불가한 것 — 보장을 문장으로

남은 25개 `expect`는 전부 상위 계층의 보장에 기댄다. 메시지를 "무엇을
기대했는지"가 아니라 "**왜 그것이 참인지**"로 다시 썼다.

| 전 | 후 |
|---|---|
| `expect("arena index fits u32")` | `expect("a file has fewer than u32::MAX nodes")` |
| `expect("still an enum")` | `expect("the id came from the enum table")` |
| `expect("checked above")` | (조건을 값으로 읽어 `if let`으로 없앴다) |
| `expect("preservation validation requires the source")` | `expect("flatten installs the source before validating against it")` |

`find(...).unwrap()` 뒤에 `any(...)`가 있던 sema의 세 곳은 문장을 다는 대신
**`if let Some(at) = ... find(...)`으로 합쳤다** — 같은 술어를 두 번 계산하던
코드였고, 합치면 패닉이 존재하지 않게 된다. 이유를 적을 필요가 없는 것이
이유를 적는 것보다 낫다.

`u32` 변환 계열(HIR 아레나, 블록, variant/field 인덱스)은 전부 "한 파일의
구문을 센다"는 같은 보장이다. 4 billion개를 담은 파일은 메모리에 올라오지도
않는다 — 그 문장을 각 자리에 적었다.

### 4. 남은 11개 raw 패닉

`ice.rs` 자신의 3개(`raise`, `bug!`, `panic_for_test`)와, 메시지로 이유를
말하는 `unreachable!` 8개다. 메시지가 없던 두 개(`codegen/core.rs:146`,
`main.rs:632`)에 이유를 달았다 — 둘 다 몇 줄 위의 가드가 보장한다.

## 작업 내역

1. `src/ice.rs`: `bug!` 매크로와 `bug_message()` 추가. 접두사 중복 금지를
   크레이트 소스 스캔 테스트로 고정.
2. 53개 사이트를 `crate::ice::bug!`로, `assert!` 메시지에서 접두사 제거,
   `server.rs`의 wire 메시지를 `ice::bug_message()`로.
3. (a) 3종 수정, (b) 25개 메시지 재작성, sema 3곳은 패닉 자체를 제거.
4. `unreachable!` 2개에 이유 추가.

## 이슈 및 해결

- **증상**: 검토 중 ICE 출력이 `internal compiler error: internal compiler
  error: ...`로 나오는 것을 발견.
- **원인**: TASK-214가 리포터에 접두사를 넣었지만, 그 전부터 있던 53개
  사이트가 메시지에 접두사를 갖고 있었다. 두 곳이 같은 문구를 소유했다.
- **해결**: 결정 1 — 소유자를 `ice.rs` 하나로, 그리고 그 사실을 테스트로.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 전체 초록

## 결과

제품 코드에 `unwrap()`이 **하나도 없다**. 남은 `expect` 25개는 각각 왜 참인지를
말하고, 의도된 ICE 53개는 `ice::bug!`라는 이름으로 자기가 무엇인지 말하며,
그중 어느 것도 접두사를 두 번 쓰지 않는다.

### 변경 파일

- `src/ice.rs`, `src/server.rs`, `src/main.rs`, `src/lib.rs`
- `src/codegen/core.rs`, `src/codegen/rope.rs`, `src/core_ir/lower.rs`
- `src/hir/lower.rs`, `src/hir/ids.rs`, `src/flow/mod.rs`
- `src/sema.rs`, `src/resolve/mod.rs`, `src/analysis/mod.rs`
- `src/engine/semantics.rs`
- `docs/tasks/INDEX.md`
