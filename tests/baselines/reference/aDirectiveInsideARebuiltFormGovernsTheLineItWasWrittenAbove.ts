//// [aDirectiveInsideARebuiltFormGovernsTheLineItWasWrittenAbove.tt] ////
variant O { A(n: number), B }
declare function g(a: number, b: number): number;
declare function h(n: number): number;
declare function two(a: string, b: string): string;
declare const obj: { m(a: number, b: number): number } | undefined;
declare const shape: { side: number };
declare let slot: number | undefined;
export function f(o: O, c: boolean) {
  const call = g(
    // @ts-expect-error
    "not a number",
    match (o) { A(n) => n, B => 0 },
  );
  const optional = obj?.m(
    // @ts-expect-error
    "not a number",
    match (o) { A(n) => n, B => 0 },
  );
  const ternary = c
    // @ts-expect-error
    ? h("s")
    : match (o) { A(n) => n, B => 0 };
  const logical = c &&
    // @ts-expect-error
    shape - match (o) { A(n) => n, B => 0 };
  slot ??=
    // @ts-expect-error
    shape - match (o) { A(n) => n, B => 0 };
  const piped = match (o) { A(n) => n, B => 0 }
    |> String
    // @ts-expect-error
    |> two;
  const arm = match (o) {
    A(n) =>
      // @ts-expect-error
      n.foo,
    B => 0,
  };
  return [call, optional, ternary, logical, piped, arm];
}


//// [aDirectiveInsideARebuiltFormGovernsTheLineItWasWrittenAbove.ts]
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
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare function g(a: number, b: number): number;
declare function h(n: number): number;
declare function two(a: string, b: string): string;
declare const obj: { m(a: number, b: number): number } | undefined;
declare const shape: { side: number };
declare let slot: number | undefined;
export function f(o: O, c: boolean) {
  let $tt_v0: number;
  const $tt_v1: typeof g = (g);
  const $tt_v2 = ("not a number");
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = $tt_v1(
    // @ts-expect-error
    $tt_v2, n);
        break;
      }
      case "B": {
        $tt_v0 = $tt_v1(
    // @ts-expect-error
    $tt_v2, 0);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const call = $tt_v0;
  let $tt_v5: (number) | (undefined);
  if (obj != null) {
    let $tt_v3: number;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v3 = n;
          break;
        }
        case "B": {
          $tt_v3 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v5 = obj?.m(
    // @ts-expect-error
    "not a number", $tt_v3);
  } else {
    $tt_v5 = undefined;
  }
  
  const optional = $tt_v5;
  let $tt_v8: number;
  if (c) {
    $tt_v8 =
    // @ts-expect-error
    h("s");
  } else {
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v8 = n;
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
  }
  
  const ternary = $tt_v8;
  let $tt_v12: (number) | (false);
  let $tt_v11: boolean;
  if ($tt_v11 = c) {
    let $tt_v9: number;
    const $tt_v10: typeof shape = (shape);
    {
      const $tt_m = o; switch ($tt_m.kind) { case "A": { const { n } = $tt_m; $tt_v9 = n; break; } case "B": { $tt_v9 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v12 = $tt_v11 &&
    // @ts-expect-error
    $tt_v10 - $tt_v9;
  } else {
    $tt_v12 = $tt_v11;
  }
  
  const logical = $tt_v12;
  if (slot == null) {
    let $tt_v13: number;
    const $tt_v14: typeof shape = (shape);
    {
      const $tt_m = o; switch ($tt_m.kind) { case "A": { const { n } = $tt_m; $tt_v13 = n; break; } case "B": { $tt_v13 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    slot =
    // @ts-expect-error
    $tt_v14 - $tt_v13; } ;
  let $tt_v17: string;
  do {
    let $tt_v20: number;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v20 = n;
          break;
        }
        case "B": {
          $tt_v20 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    const $tt_v21: string = String($tt_v20);
    // @ts-expect-error
    $tt_v17 = two($tt_v21);
    break;
  } while (false);
  const piped = $tt_v17;
  let $tt_v18;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        // @ts-expect-error
        $tt_v18 = n.foo;
        break;
      }
      case "B": {
        $tt_v18 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const arm = $tt_v18;
  return [call, optional, ternary, logical, piped, arm];
}
