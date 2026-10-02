//// [tsconfig.json] ////
{
  "compilerOptions": {
    "target": "es2022",
    "lib": ["es2022", "esnext.disposable", "dom"],
    "module": "preserve",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  }
}

//// [main.tt] ////
import type { TResult } from "@tt/std";
import * as Result from "@tt/std/result";
variant Kind { Fresh(name: string), Plain, Fail }
class Resource implements Disposable {
  constructor(readonly name: string) {
    console.log(`open ${name}`);
  }
  [Symbol.dispose]() {
    console.log(`dispose ${this.name}`);
  }
}
function declarators(kind: Kind) {
  console.log(`-- declarators ${kind.kind}`);
  using first = new Resource("first"), second = match (kind) {
    Fresh(name) => new Resource(name),
    Plain => new Resource("plain"),
    Fail => { throw new Error("no second"); },
  };
  console.log(`body ${first.name} ${second.name}`);
}
function armScope(kind: Kind) {
  console.log(`-- arm scope ${kind.kind}`);
  using outer = new Resource("outer");
  const label = match (kind) {
    Fresh(name) => {
      using inner = new Resource(name);
      console.log(`arm ${inner.name}`);
      return inner.name.toUpperCase();
    },
    Plain => "plain",
    Fail => { throw new Error("arm threw"); },
  };
  console.log(`label ${label} with ${outer.name}`);
}
function propagate(ok: boolean): TResult<string, string> {
  console.log(`-- propagate ${ok}`);
  using held = new Resource("held");
  const value = try (ok ? Result.Ok("fine") : Result.Err("bad"));
  console.log(`after try ${value} with ${held.name}`);
  return Result.Ok(value);
}
for (const kind of [Kind.Fresh("fresh"), Kind.Plain, Kind.Fail]) {
  for (const body of [declarators, armScope]) {
    try {
      body(kind);
    } catch (e) {
      console.log(`caught ${(e as Error).message}`);
    }
  }
}
console.log(JSON.stringify(propagate(true)));
console.log(JSON.stringify(propagate(false)));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [main.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
type Kind =
  | { kind: "Fresh"; name: string }
  | { kind: "Plain" }
  | { kind: "Fail" };
const Kind = {
  Fresh: (name: string): Kind => ({ kind: "Fresh", name }),
  Plain: { kind: "Plain" } as const,
  Fail: { kind: "Fail" } as const,
};
class Resource implements Disposable {
  constructor(readonly name: string) {
    console.log(`open ${name}`);
  }
  [Symbol.dispose]() {
    console.log(`dispose ${this.name}`);
  }
}
function declarators(kind: Kind) {
  console.log(`-- declarators ${kind.kind}`);
  using first = new Resource("first");
  let $tt_v0: Resource;
  {
    const $tt_m = kind;
    switch ($tt_m.kind) {
      case "Fresh": {
        const { name } = $tt_m;
        $tt_v0 = new Resource(name);
        break;
      }
      case "Plain": {
        $tt_v0 = new Resource("plain");
        break;
      }
      case "Fail": {
        throw new Error("no second");
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  using second = $tt_v0;
  console.log(`body ${first.name} ${second.name}`);
}
function armScope(kind: Kind) {
  console.log(`-- arm scope ${kind.kind}`);
  using outer = new Resource("outer");
  let $tt_v1: string;
  {
    const $tt_m = kind;
    switch ($tt_m.kind) {
      case "Fresh": {
        const { name } = $tt_m;
        using inner = new Resource(name);
      console.log(`arm ${inner.name}`);
        $tt_v1 = inner.name.toUpperCase();
        break;
      }
      case "Plain": {
        $tt_v1 = "plain";
        break;
      }
      case "Fail": {
        throw new Error("arm threw");
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const label = $tt_v1;
  console.log(`label ${label} with ${outer.name}`);
}
function propagate(ok: boolean): TResult<string, string> {
  console.log(`-- propagate ${ok}`);
  using held = new Resource("held");
  const $tt_t0 = (ok ? Result.Ok("fine") : Result.Err("bad"));
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const value = $tt_t0.value;
  console.log(`after try ${value} with ${held.name}`);
  return Result.Ok(value);
}
for (const kind of [Kind.Fresh("fresh"), Kind.Plain, Kind.Fail]) {
  for (const body of [declarators, armScope]) {
    try {
      body(kind);
    } catch (e) {
      console.log(`caught ${(e as Error).message}`);
    }
  }
}
console.log(JSON.stringify(propagate(true)));
console.log(JSON.stringify(propagate(false)));
