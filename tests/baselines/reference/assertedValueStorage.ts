//// [assertedValueStorage.tt] ////
// Repro from TASK-596
variant O { A, B }
type Ev = { kind: "click"; x: number } | { kind: "key"; code: string };
declare function read(): string;
declare const x0: unknown;
export function f(o: O, x: unknown) {
  const n = match (o) { A => x, B => 0 } as number;
  const m = match (o) { A => ({ a: 1 }), B => ({ a: 2 }) } satisfies { a?: number };
  return n + m.a.toFixed(1).length;
}
export function g(o: O) {
  const k = match (o) { A => read(), B => "b" } as "a" | "b";
  const e = match (o) { A => ({ kind: "click", x: 1 }), B => ({ kind: "key", code: "z" }) } satisfies Ev;
  const l = match (o) { A => 1, B => 2 } satisfies 1 | 2;
  let w = match (o) { A => 1, B => 2 } satisfies number;
  w = 5;
  const q: string = match (o) { A => 1, B => 2 } as unknown as string;
  const c = <number>match (o) { A => x0, B => 0 };
  return [k, e.kind, l, w, q, c];
}
export function h(o: O): number {
  return match (o) { A => x0, B => 0 } as number;
}


//// [assertedValueStorage.ts]
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
// Repro from TASK-596
type O =
  | { kind: "A" }
  | { kind: "B" };
const O = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
type Ev = { kind: "click"; x: number } | { kind: "key"; code: string };
declare function read(): string;
declare const x0: unknown;
export function f(o: O, x: unknown) {
  let $tt_v0;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v0 = x;
        break;
      }
      case "B": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const n = $tt_v0 as number;
  let $tt_v1: {
    a: number;
};
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v1 = ({ a: 1 });
        break;
      }
      case "B": {
        $tt_v1 = ({ a: 2 });
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const m = $tt_v1 satisfies { a?: number };
  return n + m.a.toFixed(1).length;
}
export function g(o: O) {
  let $tt_v2: string;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v2 = read();
        break;
      }
      case "B": {
        $tt_v2 = "b";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const k = $tt_v2 as "a" | "b";
  let $tt_v3: ({
    kind: "click";
    x: number;
}) | ({
    kind: "key";
    code: string;
});
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v3 = ({ kind: "click", x: 1 });
        break;
      }
      case "B": {
        $tt_v3 = ({ kind: "key", code: "z" });
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const e = $tt_v3 satisfies Ev;
  let $tt_v4: (1) | (2);
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v4 = 1;
        break;
      }
      case "B": {
        $tt_v4 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const l = $tt_v4 satisfies 1 | 2;
  let $tt_v5: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v5 = 1;
        break;
      }
      case "B": {
        $tt_v5 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  let w = $tt_v5 satisfies number;
  w = 5;
  let $tt_v6: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v6 = 1;
        break;
      }
      case "B": {
        $tt_v6 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const q: string = $tt_v6 as unknown as string;
  let $tt_v7;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v7 = x0;
        break;
      }
      case "B": {
        $tt_v7 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const c = <number>$tt_v7;
  return [k, e.kind, l, w, q, c];
}
export function h(o: O): number {
  let $tt_v8;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v8 = x0;
        break;
      }
      case "B": {
        $tt_v8 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v8 as number;
}
