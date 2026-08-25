# TASK-219: 방출 코드 가독성 — 블록 암 들여쓰기와 런타임 import 위치

- **상태**: 대기
- **시작일**: —
- **완료일**: —
- **커밋**: —

## 목적

TASK-215의 스냅샷 픽스처가 방출 결과를 파일로 고정하면서 두 가지가 눈에 보이게
됐다. 둘 다 계약 위반은 아니지만, 생성된 `.ts`를 사람이 연다는 점에서 품질
문제다.

**1. 블록 암 본문의 들여쓰기** (`tests/fixtures/emit/match-arm-blocks-and-guards`)

```ts
      if ($tt_m.kind === "Value") { const { n } = $tt_m; { const label = n.toFixed(1);
      $tt_v0 = `value ${label}`; break;
      } }
```

사용자 코드는 바이트 단위로 복사된다는 계약("your own code is copied
byte-for-byte and never reformatted") 그대로의 결과다. 생성된 `if (...) { ... {`
뒤에 원본 들여쓰기가 이어지면서 줄이 들쭉날쭉해진다.

**2. 런타임 import가 파일 끝에 붙는다** (`tests/fixtures/emit/pipeline-and-flow`)

```ts
export const once = $tt_ap(half(4), twice).toFixed(1);
...
import { $tt_ap, $tt_fl } from "@tt/runtime";
```

import는 호이스팅되므로 유효하지만, 파일을 여는 사람에게는 낯설다.

## 범위

- 포함:
  - 2번(런타임 import 위치)을 먼저 다룬다 — 사용자 코드를 건드리지 않고 방출
    위치만 바꾸는 문제이므로 계약과 충돌하지 않는다.
  - 1번은 계약과 정면으로 만난다: 사용자 코드를 재포맷하지 않으면서 생성된
    래퍼가 그 코드를 더 나은 자리에 놓을 수 있는지 검토한다. 예를 들어 블록 암의
    래퍼가 본문을 자기 줄에서 열도록 하는 것은 사용자 바이트를 바꾸지 않는다.
- 제외:
  - 사용자 코드의 재포맷. 계약 1·2를 깨뜨린다. 개선이 그것을 요구한다면 하지
    않는 것이 맞고, 그 판단을 문서에 남긴다.

## 의사결정

## 작업 내역

## 이슈 및 해결

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `UPDATE_EXPECT=1 cargo test --test snapshot` 후 diff를 읽고 검토

## 결과
