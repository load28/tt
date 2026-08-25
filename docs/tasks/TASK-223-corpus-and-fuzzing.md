# TASK-223: 실세계 코퍼스 차등 테스트와 퍼징

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: (아래 "작업 내역")

## 목적

AGENTS.md의 첫 번째 설계 계약은 전칭 명제다.

> 모든 유효한 TypeScript 파일은 그대로 유효한 `.tt` 파일입니다.

이 명제를 지키는 것이 지금은 `tests/passthrough.rs` **손으로 쓴 475줄**뿐이다.
전칭 명제를 손으로 쓴 사례들로 지키는 것은 품질 보장이 아니라 표본 조사다.

이 계약에는 오라클이 필요 없다는 점이 중요하다. tt 구문이 없는 파일은 정의상
**입력 바이트 == 출력 바이트**여야 하므로, 임의의 TypeScript 코퍼스를 통과시켜
비교하기만 하면 된다.

## 범위

- 포함:
  - **차등 통과 테스트** — 실제 TypeScript 코퍼스로 바이트 동일성 확인
  - **`cargo fuzz` 타깃 2개** — (a) 임의 바이트 → 패닉 금지, (b) tt 구문 생성기
  - 코퍼스를 vendoring할지 CI에서 고정 ref로 가져올지 결정
  - CI 배치: 무거우면 예약 실행으로 분리하고 PR에는 표본을 돌린다
- 제외:
  - 발견된 버그의 수정. 이 태스크는 **찾는 기계**를 만든다.

## 의사결정

### 1. "유효한 TypeScript인가"의 오라클은 통과 자체에서 나온다

이것이 이 태스크의 핵심 문제였다. 찾아온 코퍼스에는 **일부러 잘못된** 파일이
섞여 있다 — TypeScript의 `tests/cases/`는 컴파일러의 테스트 스위트이므로
문법 오류 회귀 케이스와 `// @Filename:` 다중 파일 지시자가 가득하다. 그런
파일에 대한 거부는 계약 위반이 아니다.

컴파일러 자신은 이 둘을 구분할 수 없다. 실제로 `verify-failed`의 메시지가
그렇게 말한다 — "This is either invalid TypeScript passed through from the
source or a ttc bug". 하지만 **테스트는 구분할 수 있다**:

1. 출력 자기 검사를 끄고 컴파일한다.
2. 결과가 소스와 바이트 동일하면, 자기 검사를 켜고 다시 컴파일한다.
   이제 swc에게 묻는 대상은 **소스 자신의 바이트**다.
3. swc가 거부하면 그 파일은 유효한 TypeScript가 아니다 → 대상에서 제외.
   받아들이면 계약이 지켜졌다.
4. 바이트가 달라졌거나 tt 규칙이 무언가를 주장했다면 → 보고.

"출력이 입력과 같다"를 이미 아는 쪽만 할 수 있는 추론이고, 새 공개 API도
코퍼스 큐레이션도 필요 없다. `TTC_CORPUS`로 아무 트리나 겨눠도 성립한다.

### 2. 코퍼스는 vendoring하지 않는다 — 이미 두 개가 있다

- **이 저장소 자신의 TypeScript** (`editors/vscode/server/src`,
  `website/scripts`, `integrations`). 손으로 쓴 진짜 코드이고, 항상
  존재하고, 리뷰를 거친다. 내려받을 것도 고정할 것도 없어서 **모든 기계와
  모든 잡에서** 돈다.
- **typescript-go 체크아웃** — `testdata/tests/cases`(TypeScript 자신의
  conformance 코퍼스), `internal/bundled/libs`(표준 라이브러리 선언),
  `_packages/native-preview/src`. CI가 typed 스위트를 위해 **이미 고정한**
  revision이므로 새 pin도 새 다운로드도 없다.

수천 개의 `.ts`를 저장소에 넣는 것은 리뷰할 수 없는 diff를 만든다.

### 3. PR에는 표본, 예약 실행에는 전부 — 표본은 **고정된** 것

`SAMPLE = 250`, 경로 순으로 정렬한 뒤 균등 간격(`step_by`)으로 뽑는다.
무작위 표본은 실행마다 다른 것을 테스트하므로 bisect가 불가능하다.

- `check` 잡의 `cargo test`: 저장소 자신의 TypeScript (툴체인 불필요, 공짜)
- `native` 잡: typescript-go 코퍼스의 표본 250개, `TTC_REQUIRE_CORPUS=1`
- `Soak` 워크플로(주 1회 + 수동): 전부

### 4. 퍼저는 **별도 패키지**다

`cargo fuzz`는 nightly를 요구하고(libFuzzer 계측이 nightly `-Z` 플래그),
이 저장소는 TASK-226에서 모든 게이트를 하나의 stable 버전에 고정했다.
`fuzz/`를 자기 `[workspace]`를 가진 별도 패키지로 두면 그 핀이 계속 뜻대로
동작하고, `cargo build`·`cargo test`·`cargo clippy --all-targets`가
아무도 요청하지 않은 툴체인을 건드리지 않는다.

### 5. 타깃 (b)는 "컴파일된다 + 방출이 TypeScript다"를 함께 묻는다

임의 바이트로는 흥미로운 tt 프로그램이 사실상 나오지 않는다. 그래서 (b)는
`arbitrary`로 **모양을 뽑는** 구조적 생성기다 — enum, 모든 케이스를 덮는
match, 파이프라인, `try`, `result` 블록, `if let`. 그리고 두 가지를 단언한다.

1. 컴파일된다. 언어가 허용하는 프로그램을 거부하면 그것이 버그다.
2. 방출이 TypeScript로 파싱된다. `compile`이 이미 하는 검사이므로 켜두기만
   하면 된다.
3. 덤으로 **결정성** — 같은 입력을 두 번 컴파일하면 같은 출력. 빌드가
   방출을 캐시할 수 있게 하는 성질이다.

`tsc`를 루프 안에서 돌리지 않는다: 프로세스당 수백 ms는 퍼징을 무의미하게
만들고, swc 기반 자기 검사가 같은 질문(파싱되는가)에 in-process로 답한다.

## 작업 내역

1. `tests/corpus.rs` — 차등 통과 테스트. 결정 1의 판정, 결정 2의 기본 코퍼스,
   결정 3의 고정 표본, `TTC_CORPUS`/`TTC_CORPUS_FULL`/`TTC_REQUIRE_CORPUS`.
   측정 결과를 항상 출력한다 — 전부 skip한 실행은 통과한 실행과 똑같이 보이므로.
2. `fuzz/` — 별도 패키지, 타깃 2개.
   `fuzz_targets/compile_any_bytes.rs`(임의 텍스트 → `analyze`/`compile`,
   두 `SourceKind` 모두), `fuzz_targets/generated_tt_compiles.rs`(구조적 생성기).
3. `.github/workflows/ci.yml` — `native` 잡에 표본 코퍼스 단계 추가.
4. `.github/workflows/soak.yml` — 주 1회 전체 코퍼스 + 두 퍼저(각 15분),
   크래시 입력을 artifact로 남긴다.

## 이슈 및 해결

- **증상**: 첫 실행에서 `testdata/tests/cases`의 파일 수십 개가 실패.
- **원인**: 그 코퍼스는 컴파일러의 **부정 테스트** 모음이다. `// @Filename:`
  다중 파일 지시자, 문법 오류 회귀 케이스가 섞여 있다.
- **해결**: 결정 1. 필터를 넣은 뒤 같은 코퍼스에서 363개 중 306개 통과,
  57개 정확히 제외, 0개 위반.

- **증상**: 구조적 퍼저가 26초 만에 "잘 형성된 tt 프로그램이 거부됐다"를 보고.
- **원인**: **생성기**가 틀렸다. 단위 케이스에 `if let C0 = e`를 만들었는데,
  tt에서 패턴 괄호는 필수다 — 괄호가 없으면 `C0`는 케이스가 아니라 `e`를
  묶는 **새 이름**이 되므로 뜻이 달라진다.
- **해결**: 생성기가 `C0()`를 쓰도록 고쳤다. 타깃이 엄격했던 것이 맞고,
  고칠 것은 컴파일러가 아니라 생성기였다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 전체 초록 (TASK-227 수정 후)
- [x] 코퍼스 실행이 초록이거나, 실패가 전부 태스크로 등록됨 →
      **TASK-227로 등록하고 같은 브랜치에서 고쳤다**
- [x] `cargo +nightly fuzz build` — 두 타깃 모두 빌드
- [x] `compile_any_bytes` 91초 / 152,437회 — 크래시 없음
- [x] `generated_tt_compiles` 151초 / 26,138회 — 생성기 수정 후 발견 없음

## 결과

기본 코퍼스 594개 파일 중 **533개가 유효한 TypeScript이고 전부 바이트 동일**,
60개는 유효한 TypeScript가 아니어서 제외, **1개가 계약 위반**.

그 하나가 이 기계의 값어치다: `website/scripts/essay.ts` — 이 저장소가
스스로 빌드하는, 리뷰를 거친 유효한 TypeScript 파일을 ttc가 거부한다.

```ts
for (const match of xs) { ... }
```

`match`가 바인딩 이름인 경우다. 손으로 쓴 `passthrough.rs`는 메서드 이름과
프로퍼티는 고정하고 있었지만 바인딩 이름은 빠뜨렸다 — 표본 조사가 놓치는
것이 정확히 이런 종류다. **TASK-227**로 등록했다.

### 변경 파일

- `tests/corpus.rs` (신규)
- `fuzz/Cargo.toml`, `fuzz/.gitignore`,
  `fuzz/fuzz_targets/compile_any_bytes.rs`,
  `fuzz/fuzz_targets/generated_tt_compiles.rs` (신규)
- `.github/workflows/ci.yml`, `.github/workflows/soak.yml`
- `docs/tasks/TASK-227-match-claimed-as-a-binding-name.md` (신규)
- `docs/tasks/INDEX.md`
