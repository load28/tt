# TASK-200: 일반 compile 출력용 표준 source map

- **상태**: 대기
- **시작일**: —
- **완료일**: —
- **커밋**: —

## 목적

TASK-160이 lowering 의미 보존과 validator를 완성했으므로, 이제 일반 `ttc`
출력이 표준 source map을 함께 내보내 Node stack trace와 브라우저 debugger가
`.tt`/`.ttx` 원본을 가리키게 한다. TASK-160 지시 §9가 "먼저 lowering 의미
보존과 validator를 완성한다"는 조건으로 분리해 둔 후속 과제다.

## 범위

- 포함: 표준 source map(v3) 생성과 `compile` API의 code+map 동시 제공,
  Node stack trace / 브라우저 debugger / 번들러(Vite·Rolldown·Rollup·esbuild)
  합성 검증, 생성 helper 내부 오류를 가장 가까운 TT construct로 연결.
- 제외: lowering 형태 변경, 새 최적화, 언어 표면 변경.

## 근거 자료 (TASK-160이 남긴 기반)

map은 **출력 문자열을 검색해서 만들지 않는다.** 다음 구조를 직접 소비한다.

- `codegen::rope::Rope` / `TargetPiece` — 원본 조각과 생성 조각의 구분
- `SourceOrigin` (`Exact` / `Construct` / `Synthetic`) — 조각별 provenance
- `EmitMapping` — 원본↔출력 바이트의 양방향 대응
- `EmitAnchor` — 생성 glue를 쓴 construct의 소유권
- `SourcePreservation` 의 `owned` / `relocated` / `rewritten` — 어떤 원본
  구간이 그대로·이동·재작성되었는지에 대한 계획된 사실

## 의사결정

(작업 시작 시 기록)

## 작업 내역

(작업 시작 시 기록)

## 이슈 및 해결

(작업 시작 시 기록)

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `TTC_REQUIRE_TSGO=1 cargo test`

## 결과

대기.
