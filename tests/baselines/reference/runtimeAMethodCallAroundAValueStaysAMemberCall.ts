//// [runtimeAMethodCallAroundAValueStaysAMemberCall.tt] ////

const trace: string[] = [];
variant K { A, B }
function pick(k: K): K { trace.push("pick"); return k; }
class Repo {
  items = ["x"];
  first<T>(this: Repo, fallback: T): string | T { trace.push("first"); return this.items[0] ?? fallback; }
}
type O = { name: string; id<T>(x: T): T; add(x: number): number };
const obj: O = { name: "o", id: (x) => x, add(x: number) { return x + 1; } };
function getO(): O { trace.push("getO"); return obj; }
function key(): "add" { trace.push("key"); return "add"; }
function repo(k: K, r: Repo) {
  const v: string | number = r.first(match (pick(k)) { A => 1, B => 2 });
  return v;
}
function optional(o: O | undefined, k: K) {
  const s: string | undefined = o?.id(match (pick(k)) { A => o.name, B => "b" });
  return s;
}
function keyed(k: K) {
  return getO()[key()](match (pick(k)) { A => 1, B => 2 });
}
const n: number = obj.id(match (pick(K.B)) { A => 1, B => 2 });
console.log(repo(K.A, new Repo()), optional(obj, K.A), optional(undefined, K.A), keyed(K.B), n, JSON.stringify(trace));

export {};


//// [runtimeAMethodCallAroundAValueStaysAMemberCall.ts]
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

const trace: string[] = [];
type K =
  | { kind: "A" }
  | { kind: "B" };
const K = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
function pick(k: K): K { trace.push("pick"); return k; }
class Repo {
  items = ["x"];
  first<T>(this: Repo, fallback: T): string | T { trace.push("first"); return this.items[0] ?? fallback; }
}
type O = { name: string; id<T>(x: T): T; add(x: number): number };
const obj: O = { name: "o", id: (x) => x, add(x: number) { return x + 1; } };
function getO(): O { trace.push("getO"); return obj; }
function key(): "add" { trace.push("key"); return "add"; }
function repo(k: K, r: Repo) {
  let $tt_v0: number;
  {
    const $tt_m = pick(k);
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      case "B": $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  const v: string | number = r.first(($tt_v0 === 0 ? 1 : 2));
  return v;
}
function optional(o: O | undefined, k: K) {
  let $tt_v4: string | undefined;
  if (o != null) {
    {
      const $tt_m = pick(k);
      switch ($tt_m.kind) {
        case "A": {
          $tt_v4 = o?.id(o.name);
          break;
        }
        case "B": {
          $tt_v4 = o?.id("b");
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v4 = undefined;
  }
  
  const s: string | undefined = $tt_v4;
  return s;
}
function keyed(k: K) {
  let $tt_v5: number;
  const $tt_v7 = (getO());
  const $tt_v8 = (key());
  {
    const $tt_m = pick(k);
    switch ($tt_m.kind) {
      case "A": $tt_v5 = 0; break;
      case "B": $tt_v5 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return $tt_v7[$tt_v8](($tt_v5 === 0 ? 1 : 2));
}
let $tt_v9: number;
{
  const $tt_m = pick(K.B);
  switch ($tt_m.kind) {
    case "A": $tt_v9 = 0; break;
    case "B": $tt_v9 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
const n: number = obj.id(($tt_v9 === 0 ? 1 : 2));
console.log(repo(K.A, new Repo()), optional(obj, K.A), optional(undefined, K.A), keyed(K.B), n, JSON.stringify(trace));

export {};
