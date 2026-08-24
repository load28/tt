# TASK-200: 일반 compile 출력용 표준 source map

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: cba5df7

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

### 결정 1: map은 emission이 이미 기록한 사실에서만 만든다

- **상황**: 출력의 어느 바이트가 원본의 어디에서 왔는지를 알아야 한다.
- **검토한 대안**: 출력 문자열을 스캔해 식별자·구문 모양으로 원본을 추정하는
  방식 — 지시가 금지한 문자열 휴리스틱이고, 생성 glue에는 대응되는 원본
  텍스트가 아예 없어 원리적으로 답을 못 낸다.
- **선택과 근거**: `EmitMapping`(그대로 복사된 원본 구간)과
  `EmitAnchor`(각 glue를 쓴 construct)만 소비한다. 두 구조는 이미 진단이
  쓰는 것과 같은 답이므로, "스택 프레임이 가리키는 곳"과 "진단이 가리키는
  곳"이 구조적으로 일치한다. 방출 텍스트는 줄 수와 UTF-16 열을 세기 위해서만
  읽는다 — 그것이 이 포맷의 좌표 정의다.

### 결정 2: 배너는 재측정하지 않고 선언한다

- **상황**: CLI가 emission 이후 `// @generated` 한 줄을 앞에 붙이므로 모든
  출력 좌표가 밀린다.
- **검토한 대안**: 최종 텍스트에서 배너를 다시 찾아 길이를 재는 방식 — 배너
  문안이 바뀌면 조용히 어긋난다.
- **선택과 근거**: `SourceMapRequest.generated_line_offset`으로 "앞에 몇 줄을
  붙일 것인지"를 호출자가 선언한다. map builder는 그 수만큼 `;`을 앞에 둘
  뿐이고, 사실의 출처가 하나다.

### 결정 3: `sources` 경로는 파일시스템이 아니라 경로 자체에서 계산한다

- **상황**: map은 자기 디렉터리 기준 상대 경로로 원본을 가리켜야 한다.
- **검토한 대안**: `canonicalize()`로 두 절대 경로를 얻어 상대화 — 첫 빌드에는
  출력 디렉터리가 아직 없어 실패하고, 그때 원본 경로를 그대로 쓰는 폴백이
  **틀린 경로를 그럴듯하게** 만든다 (이슈 1).
- **선택과 근거**: `.`과 `..`만 접는 순수 lexical 정규화로 계산한다. 심볼릭
  링크를 해석하지 않고 파일 존재 여부에 의존하지 않으므로, 첫 빌드든 재빌드든
  같은 답이 나온다. 이것이 map 소비자가 경로를 다루는 모델과도 같다.

### 결정 4: map은 ttc가 실제로 변환한 파일에만 붙인다

- **상황**: `--source-map file`을 켜면 손으로 쓴 `.ts`에도 map이 생기고
  `//# sourceMappingURL=` 줄이 붙는다 (이슈 2).
- **검토한 대안**: 옵션이 켜졌으니 모든 출력에 붙인다 — 불변 원칙 1
  ("유효한 TypeScript는 바이트 그대로 통과")을 정면으로 깬다.
- **선택과 근거**: map은 *번역*을 기술하는 문서다. 그대로 복사되는 파일에는
  기술할 번역이 없고, 그 경우가 바로 계약이 바이트 변경을 금지하는 경우다.
  `SourceKind::from_tt_path`가 "ttc가 변환하는 표면"을 이미 판별하므로, 그
  사실을 그대로 쓴다. 특수 케이스가 아니라 map의 정의다.

### 결정 5: 기본값은 `off`, 세 모드는 소비자가 고른다

- **상황**: 맵을 항상 켜면 모든 출력에 한 줄이 늘고, 파이프로 읽는 소비자는
  별도 파일을 받을 수 없다.
- **선택과 근거**: `off`(기본) / `file`(`<output>.map` + 상대 URL) /
  `inline`(`data:` URL). `inline`은 stdout 한 줄기만 있는 `ttc -p`에서도
  맵이 함께 흐르는 유일한 형태이고, unplugin이 이를 다시 분리해 번들러에
  객체로 넘긴다.

### 결정 6: 세그먼트 입도는 cut point 단위다

- **상황**: 포맷 소비자는 세그먼트 사이를 보간하지 않는다.
- **선택과 근거**: 각 verbatim 구간의 시작·끝과 모든 출력 줄머리에 세그먼트를
  둔다. 구간 안에서는 양쪽이 바이트 단위로 나란히 나아가므로 **줄은 정확**하고
  열은 그 조각의 시작을 가리킨다. tsc가 노드 단위로 매핑하는 것과 같은 성격의
  근사이며, 이 사실을 모듈 문서에 명시했다.

## 작업 내역

- 2026-08-24: `src/source_map.rs`를 추가했다 — `SourceMapRequest`,
  `SourceMap`(`to_json`/`to_data_url`/`url_comment`), Base64 VLQ 인코더,
  UTF-16 열을 세는 `LineTable`, 그리고 `EmitMapping`→구간, `EmitAnchor`→
  construct 순으로 답하는 `source_byte_at`.
- 2026-08-24: `MappedEmit::source_map(source, request)`를 공개 API로 붙였다.
  `compile`/`compile_mapped` 시그니처는 그대로다 (호환성 유지).
- 2026-08-24: CLI에 `--source-map <off|file|inline>`을 추가하고, `-o` 경로에
  `<output>.map` 쓰기와 `//# sourceMappingURL=` 부착을 연결했다.
- 2026-08-24: `emit_adt`가 쓰던 union type·생성자 객체에 `AnchorKind::Enum`
  anchor를 붙였다. 그 전에는 lowered enum 전체가 아무 곳에도 매핑되지 않아
  생성자 안의 프레임이 생성 파일을 가리켰다.
- 2026-08-24: `integrations/unplugin/index.js`가 `--source-map inline`을
  요청하고 `detachInlineSourceMap`으로 분리해 `{ code, map }`을 반환하도록
  했다 (`sourcemap: false`로 해제). 이전에는 항상 `map: null`이었다.
- 2026-08-24: 검증 — 단위 6건(VLQ, UTF-16 열, 구간/anchor 조회, 배너 shift,
  문서 형태), CLI 5건(기본 off, file 배치와 `sources`, inline 왕복, passthrough
  바이트, 배너 shift), runtime 2건(Node `--enable-source-maps` 스택이 `.tt`
  줄·열을 가리킴 / 생성 guard가 `match`로 연결됨). 실제 실행 결과:
  `at area (…/src/app.tt:5:20)`.

## 이슈 및 해결

### 이슈 1: `sources`가 첫 빌드에서만 틀린 경로가 됨

- **증상**: `ttc -o out --source-map file src/app.tt`이 `"sources":["src/app.tt"]`
  를 기록했다. map은 `out/`에 있으므로 `out/src/app.tt`로 해석되어 존재하지
  않는다. 재빌드하면(=`out/`이 이미 있으면) 올바른 `../src/app.tt`가 나왔다.
- **원인**: 상대화가 `canonicalize()`에 의존했는데, map은 `write_output`이
  디렉터리를 만들기 **전에** 만들어진다. 실패 시 원본 경로를 그대로 쓰는
  폴백이 있어 오류 없이 틀린 값이 나왔다 — 원칙 3이 금지하는 형태의 폴백이다.
- **해결**: 결정 3. 파일시스템을 읽지 않는 lexical 정규화로 바꿨다.
  CLI 테스트가 `src/`와 `out/`을 분리한 첫 빌드로 이 경로를 고정한다.

### 이슈 2: passthrough `.ts`에 `sourceMappingURL`이 붙어 바이트 계약이 깨짐

- **증상**: `--source-map file`로 손으로 쓴 `.ts`를 빌드하면 출력이 입력과
  달라졌다(`+ //# sourceMappingURL=plain.ts.map`). 불변 원칙 1 위반.
- **원인**: map 생성이 "옵션이 켜졌는가"만 보고 "이 파일을 변환하기는 했는가"를
  보지 않았다.
- **해결**: 결정 4. `SourceKind::from_tt_path`로 변환 대상만 map을 받는다.

### 이슈 3: lowered enum이 어디에도 매핑되지 않음

- **증상**: 생성 파일 앞부분(union type과 생성자 객체)에 세그먼트가 하나도
  없어, 그 줄들의 프레임이 `.tt`가 아니라 생성 파일을 가리켰다.
- **원인**: `emit_adt`가 만든 glue에 anchor가 없었다. 다른 모든 construct는
  `Rope::anchored`로 감싸는데 enum만 예외였다.
- **해결**: `AnchorKind::Enum`을 추가하고 선언 이름 span으로 anchor했다.

### mutation 검증

| mutation | 기대 검출 | 결과 |
|---|---|---|
| S1 배너 offset 무시 | 배너 shift CLI 테스트 | 실패(검출) |
| S2 열을 바이트로 계산 | UTF-16 단위 단위 테스트 | 실패(검출) |
| S3 glue를 매핑하지 않음 | 단위 + runtime 스택 테스트 | 둘 다 실패(검출) |
| S4 passthrough에도 map 부착 | passthrough 바이트 CLI 테스트 | 실패(검출) |

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` — 13개 스위트 전부 ok, 실패 0

## 결과

완료.

- `node --enable-source-maps`에서 스택 프레임이 `.tt`의 줄·열을 가리킨다
  (arm 안의 `throw` → `app.tt:5:20`).
- 브라우저 debugger는 `sourcesContent`가 실려 있어 경로 해석 없이도 원본을
  표시한다.
- 번들러(Vite·Rolldown·Rollup·esbuild)는 unplugin이 넘기는 map 객체로 자신의
  변환과 합성한다.
- 생성 helper 내부 오류가 가장 가까운 TT construct로 연결된다 (소진 guard의
  throw → 그 `match`).
- `compile`/`compile_mapped` 시그니처는 그대로고, map은
  `MappedEmit::source_map`으로 함께 제공된다.

### 남은 범위

- `names` 필드는 비워 둔다. 생성 이름과 원본 식별자의 대응은 추측 없이는
  만들 수 없고, 없다고 해서 스택·디버거 동작이 나빠지지 않는다.
- 세그먼트 열 정밀도는 결정 6의 근사다. 더 촘촘히 하려면 토큰 경계 사실이
  필요하며, 그것이 필요해지는 소비자가 생기면 별도로 다룬다.
