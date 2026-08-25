# TASK-220: 진단 렌더러의 ANSI 색상

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: (아래 "작업 내역")

## 목적

TASK-213 결정 3에서 의도적으로 미룬 항목이다. 그때의 근거는 "색상은 렌더 결과의
모든 바이트에 영향을 주므로, 스냅샷 게이트가 자리잡기 전에 넣으면 회귀를 고정할
수단 없이 표면만 넓힌다"였다. TASK-215로 그 게이트가 생겼으므로 선행 조건이
충족됐다.

rustc 수준의 가독성에서 색상이 담당하는 몫은 작지 않다 — severity, 규칙 코드,
캐럿, `help:` 라벨이 서로 다른 층이라는 것을 구조가 아니라 색이 먼저 알려준다.

## 범위

- 포함:
  - `src/render.rs`에 스타일 계층 추가. 렌더러는 스타일 집합을 받고, 그 집합이
    비어 있으면 지금과 **바이트 단위로 동일한** 출력을 낸다.
  - 색상을 켜는 조건: stdout/stderr가 터미널이고 `NO_COLOR`가 설정되지 않았을 때.
  - 파이프로 캡처되는 테스트·CI·빌드 로그는 자동으로 무색이 되므로
    `tests/fixtures/`의 기대 파일은 그대로 유효해야 한다 — 그것이 이 태스크의
    회귀 안전망이다.
- 제외:
  - `--color` 플래그. 환경 변수와 tty 감지로 충분한지 먼저 확인하고, 필요하면
    별도로 다룬다.

## 의사결정

### 1. 스타일은 렌더러의 **인자**이지 전역 상태가 아니다

`Styles`는 역할별 SGR 문자열 6개(`error`, `warning`, `message`, `gutter`,
`help`, `reset`)를 담은 `Copy` 구조체이고, 공개 렌더 함수 4개가 모두 이것을
받는다. 렌더러 안에서 tty를 감지하지 않는 이유는 두 가지다.

- 렌더 결과가 어디로 가는지는 **호출자만 안다**. 같은 함수가 stderr로 그리는
  그림과 픽스처 파일로 들어가는 텍스트를 모두 만든다.
- 인자로 두면 "무색"이 테스트에서 **명시적인 선택**이 된다.
  `tests/snapshot.rs`가 `Styles::PLAIN`을 적어 넣는 것은, 픽스처가 터미널
  환경과 무관하다는 계약을 코드가 말하는 것이다. 전역 감지였다면 그 계약은
  "CI가 tty가 아니라서 우연히 통과하는" 것이 된다.

`Styles::PLAIN`은 모든 필드가 빈 문자열이고, `paint()`가 빈 스타일에 대해
텍스트를 그대로 돌려주므로 이스케이프가 한 바이트도 섞이지 않는다. 이것이
"비어 있으면 동일"의 구현이다.

### 2. 무엇을 칠하는가 — rustc가 나누는 층 그대로

| 층 | 색 | 무엇 |
|---|---|---|
| severity | bold red / bold yellow | `error[rule]`, `warning[rule]`, 캐럿, 여러 줄 span의 괄호 |
| message | bold | 헤더 문장 |
| gutter | bold blue | `-->`, `|` 막대, 줄 번호 |
| help | bold cyan | `help:` 라벨 |

캐럿과 여러 줄 span의 `|` 괄호를 gutter가 아니라 severity로 칠한 것은 그것이
**틀이 아니라 지목**이기 때문이다. 어느 줄이 문제의 구문인지가 색으로 먼저
읽힌다.

### 3. 감지는 stderr 하나만 본다

이 바이너리가 진단을 쓰는 곳은 전부 stderr다(`--print`의 코드만 stdout으로
간다). 그래서 질문은 하나다: stderr가 터미널인가, 그리고 `NO_COLOR`가
비어 있지 않게 설정돼 있는가(<https://no-color.org>).

`main.rs`의 `styles()`는 `OnceLock`으로 프로세스당 한 번만 결정한다. 병렬 잡이
각자 감지하면 같은 실행에서 어떤 진단은 칠해지고 어떤 진단은 안 칠해질 수
있는데, 그것은 감지 비용 문제가 아니라 일관성 문제다.

`--color` 플래그는 넣지 않았다. 범위의 판단대로, tty 감지와 `NO_COLOR`로 실제
사용 경로(터미널, 파이프, 리다이렉트, CI 로그)가 모두 덮인다.

### 4. 여러 줄 편집의 그림은 색과 무관하게 같은 구조

TASK-216이 넣은 여러 줄 `= help:` 블록도 `|` 막대를 쓰므로 gutter로 칠해진다.
색을 벗기면 이전과 같은 텍스트라는 것을 `strip()` 테스트가 확인한다.

## 작업 내역

1. `src/render.rs`: `Styles`(+ `PLAIN`, `ANSI`, `for_stderr`, `for_terminal`,
   `is_plain`, 내부 `paint`/`severity`) 추가.
2. 렌더 경로 전체에 스타일을 통과시켰다 — `render`, `write_single_line`,
   `write_multi_line`, `write_suggestions`, 그리고 `bar()`/`numbered_bar()`로
   막대 작성을 한곳에 모았다.
3. 공개 함수 4개(`render`, `diagnostic`, `compile_error`, `engine_diagnostic`)에
   `styles: Styles` 인자 추가.
4. `src/main.rs`: `styles()` — `OnceLock`으로 프로세스당 한 번 결정 — 을 넣고
   진단 출력 3곳이 그것을 쓴다.
5. `tests/snapshot.rs`가 `Styles::PLAIN`을 명시한다.
6. 단위 테스트 4개: 무색이 바이트 동일, 층별 색이 실제로 붙음, warning이 자기
   색을 가짐, 여러 줄 span도 색을 벗기면 동일.

## 이슈 및 해결

이 태스크에서 새로 드러난 문제는 없었다. 기존 렌더 단위 테스트 9개와 픽스처
4묶음이 모두 `Styles::PLAIN`으로 그대로 통과했고, 그것이 "색상은 표면만
넓히지 않는다"의 증거다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test` — 전체 초록 (doc 테스트 28 포함)
- [x] 파이프 출력이 무색이고 `tests/fixtures/`의 기대 파일이 변경되지 않음
      (`git diff --stat tests/fixtures/`가 비어 있음)
- [x] pty에서 색이 나오고(`script -qec "ttc --check ..."`),
      `NO_COLOR=1`이면 같은 pty에서 무색

## 결과

터미널에서 진단이 rustc처럼 읽힌다 — severity, 틀, 지목, 조언이 색으로 먼저
갈린다. 파이프·리다이렉트·CI 로그는 예전과 바이트 단위로 동일하다.

### 변경 파일

- `src/render.rs`
- `src/main.rs`
- `tests/snapshot.rs`
- `docs/tasks/INDEX.md`
