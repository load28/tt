//// [anAnnotationNamesTheDeclarationsItsTypeRefersTo.tt] ////

variant K { A, B }
interface Item { a: number }
function outer<T>(a: T) {
  function inner<T>(b: T, k: K) {
    const z = match (k) { A => a, B => a };
    return [z, b] as const;
  }
  return inner("s", K.A);
}
function local(i: Item, k: K) {
  interface Item { b: string }
  const x: Item = { b: "s" };
  const z = match (k) { A => i, B => i };
  const y = match (k) { A => x, B => x };
  return [z.a, y.b];
}
console.log(JSON.stringify([outer(1), local({ a: 2 }, K.B)]));

export {};


//// [anAnnotationNamesTheDeclarationsItsTypeRefersTo.ts]
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

type K =
  | { kind: "A" }
  | { kind: "B" };
const K = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
interface Item { a: number }
function outer<T>(a: T) {
  function inner<T>(b: T, k: K) {
    let $tt_v0;
    {
      const $tt_m = k;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v0 = a;
          break;
        }
        case "B": {
          $tt_v0 = a;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    const z = $tt_v0;
    return [z, b] as const;
  }
  return inner("s", K.A);
}
function local(i: Item, k: K) {
  interface Item { b: string }
  const x: Item = { b: "s" };
  let $tt_v1;
  {
    const $tt_m = k;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v1 = i;
        break;
      }
      case "B": {
        $tt_v1 = i;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const z = $tt_v1;
  let $tt_v2: Item;
  {
    const $tt_m = k;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v2 = x;
        break;
      }
      case "B": {
        $tt_v2 = x;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const y = $tt_v2;
  return [z.a, y.b];
}
console.log(JSON.stringify([outer(1), local({ a: 2 }, K.B)]));

export {};
