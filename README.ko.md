# tt

[공식 홈페이지](https://load28.github.io/tt/) · [English](./README.md) · [한국어](./README.ko.md)

tt은 TypeScript에 표현력 높은 데이터·제어 흐름 기능을 더하고, 다시 순수 TypeScript로 컴파일하는 작은 언어입니다.

> [!WARNING]
> **개발 중:** tt은 아직 프로덕션 사용을 권장하지 않습니다. 릴리스 사이에 API와 언어 동작이 바뀔 수 있습니다.

```tt
export variant Shape {
  Circle(radius: number),
  Rectangle(width: number, height: number),
  Point,
}

export const area = (shape: Shape): number =>
  match (shape) {
    Circle(radius) => Math.PI * radius ** 2,
    Rectangle(width, height) => width * height,
    Point => 0,
  };
```

모든 유효한 TypeScript 파일은 그대로 유효한 `.tt` 파일이고, 모든 유효한 TSX 파일은 유효한 `.ttx` 파일입니다. tt은 자신이 소유한 구문만 변환하고, 소진되지 않은 match 같은 언어 오류를 직접 보고하며, 런타임 의존성 없는 읽기 쉬운 TypeScript 또는 TSX를 방출합니다.

## 시작하기

### 새 프로젝트

```sh
bunx @load28/create-tt@next my-app
cd my-app
bun run dev
```

### 기존 TypeScript 프로젝트

```sh
cd existing-project
bunx @load28/create-tt@next init
bun run tt:check
```

[설치 가이드](./docs/getting-started.ko.md)에서 자동 설치와 번들러별 수동 절차를
확인할 수 있습니다.

### VS Code 확장

최신 [GitHub Releases](https://github.com/load28/tt/releases) pre-release의
`tt-language-<버전>.vsix`를 설치합니다.

```sh
code --install-extension ./tt-language-<버전>.vsix
code --install-extension ./tt-typescript-preview-<버전>-<플랫폼>.vsix
```

두 VSIX를 설치한 뒤 에디터에서 TypeScript 7을 사용하도록 설정합니다.

```jsonc
// .vscode/settings.json (또는 사용자 설정)
"js/ts.experimental.useTsgo": true,
"typescript.experimental.useTsgo": true
```

### CLI 사용

```sh
bun add -d @load28/tt-lang@next typescript@7.1.0-dev.20260826.1
bunx ttc -o build src        # TypeScript 방출
bunx ttc --check src         # tt 검사
bunx ttc --check-types src   # tt + TypeScript 검사
```

- 출력: `.tt` → `.ts`, `.ttx` → `.tsx`
- 번들러: Vite, Rollup, webpack, Rspack, esbuild, Farm용 [`@load28/unplugin-tt`](./integrations/unplugin)
- 도움말: `ttc --help`, `ttc help <topic>`

## 언어 한눈에 보기

- Rust 스타일 `variant`로 데이터를 모델링하고, 가드·튜플·리터럴·or-패턴·중첩 패턴을 지원하는 소진적 `match`로 값을 추출합니다. TypeScript `enum`은 평범한 TypeScript로 그대로 둡니다.
- `@tt/std`의 `TOption`과 `TResult`를 `try`, `let-else`, `if let`, `result` 블록으로 다룹니다. 트리셰이킹 가능한 연산은 `@tt/std/option`과 `@tt/std/result`에서 가져옵니다.
- `|>`와 `flow`로 값·함수 파이프라인을 만들고, `value |> ?.name` 같은 JavaScript 방식 optional postfix step을 사용할 수 있습니다.
- 변경을 허용하지 않을 바인딩과 매개변수에는 `val`을 붙입니다.

나머지는 모두 평범한 TypeScript입니다. 기존 TypeScript의 타입, 모듈, 도구, 런타임 동작을 그대로 기반으로 사용합니다.

언어를 만들게 된 배경은 [tt를 만든 이유](./docs/why-tt.ko.md)에 적어 두었습니다.

## tt 개발하기

필요한 도구는 Rust 1.98, Node.js, Bun입니다.

```sh
git clone https://github.com/load28/tt.git
cd tt
npm ci
./scripts/setup
./scripts/ci
```

- 기여 절차: [CONTRIBUTING.md](./CONTRIBUTING.md)
- 아키텍처: [`docs/design`](./docs/design)

컴파일러는 Rust 라이브러리로도 포함할 수 있습니다.

```rust
use ttc::{compile, Options};

let typescript = compile(source, &Options::default())?;
```

## 라이선스

[MIT](./LICENSE)
