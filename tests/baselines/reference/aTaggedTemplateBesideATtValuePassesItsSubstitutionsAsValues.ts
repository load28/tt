//// [aTaggedTemplateBesideATtValuePassesItsSubstitutionsAsValues.tt] ////
function tag(s: TemplateStringsArray, ...v: unknown[]) { return v; }
variant V { A(n: number), B }
export function plain(x: number, v: V) {
  return tag`${x}${match (v) { A(n) => n, B => -1 }}`;
}
export function grouped(x: number, v: V) {
  return tag`${(x)}${match (v) { A(n) => n, B => -1 }}`;
}
export function branch(c: boolean, x: number, v: V) {
  return c ? tag`${(x)}${match (v) { A(n) => n, B => -1 }}` : null;
}
console.log(JSON.stringify([plain(5, V.A(1)), grouped(6, V.B), branch(true, 7, V.A(2))]));


//// [aTaggedTemplateBesideATtValuePassesItsSubstitutionsAsValues.ts]
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
function tag(s: TemplateStringsArray, ...v: unknown[]) { return v; }
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
export function plain(x: number, v: V) {
  let $tt_v0: number;
  const $tt_v1: typeof tag = (tag);
  const $tt_v2: typeof x = (x);
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = n;
        break;
      }
      case "B": {
        $tt_v0 = -1;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v1`${$tt_v2}${$tt_v0}`;
}
export function grouped(x: number, v: V) {
  let $tt_v3: number;
  const $tt_v4: typeof tag = (tag);
  const $tt_v5: typeof x = (x);
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v3 = n;
        break;
      }
      case "B": {
        $tt_v3 = -1;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v4`${($tt_v5)}${$tt_v3}`;
}
export function branch(c: boolean, x: number, v: V) {
  let $tt_v10: (unknown[]) | (null);
  if (c) {
    let $tt_v6: number;
    const $tt_v7: typeof tag = (tag);
    const $tt_v8: typeof x = (x);
    {
      const $tt_m = v;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v6 = n;
          break;
        }
        case "B": {
          $tt_v6 = -1;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v10 = $tt_v7`${($tt_v8)}${$tt_v6}`;
  } else {
    $tt_v10 = null;
  }
  
  return $tt_v10;
}
console.log(JSON.stringify([plain(5, V.A(1)), grouped(6, V.B), branch(true, 7, V.A(2))]));
