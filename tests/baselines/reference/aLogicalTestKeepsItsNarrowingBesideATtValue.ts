//// [aLogicalTestKeepsItsNarrowingBesideATtValue.tt] ////
variant V { A, B }
const L: string[] = [];
export function ifAnd(x: string | number, v: V) {
  if (typeof x === "string" && match (v) { A => true, B => false }) {
    return x.toUpperCase();
  }
  return "-";
}
export function ifAndElse(x: string | number, v: V) {
  if (typeof x === "number" && match (v) { A => true, B => false }) { return x.toFixed(1); } else { return String(x); }
}
export function ifOr(y: string | null, v: V) {
  if (y === null || match (v) { A => false, B => true }) return 0;
  return y.length;
}
export function loops(x: string | number, v: V) {
  let n = 0;
  while (typeof x === "string" && match (v) { A => n < 2, B => false }) { L.push(x.toUpperCase()); n++; }
  for (let i = 0; typeof x === "string" && match (v) { A => i < 1, B => false }; i++) { L.push(x.toLowerCase()); }
  let k = 0;
  while (typeof x === "number" || match (v) { A => k++ < 1, B => false }) { L.push(String(x)); if (typeof x === "number") break; }
  return L.join();
}
console.log(ifAnd("ab", V.A), ifAnd("ab", V.B), ifAnd(1, V.A));
console.log(ifAndElse(2, V.A), ifAndElse(2, V.B), ifAndElse("s", V.A));
console.log(ifOr("abc", V.A), ifOr("abc", V.B), ifOr(null, V.A));
console.log(loops("Ab", V.A), loops(3, V.B));


//// [aLogicalTestKeepsItsNarrowingBesideATtValue.ts]
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
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const L: string[] = [];
export function ifAnd(x: string | number, v: V) {
  let $tt_v2: boolean;
  if (typeof x === "string") {
    let $tt_v0: boolean;
    {
      const $tt_m = v;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v0 = true;
          break;
        }
        case "B": {
          $tt_v0 = false;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v2 = $tt_v0;
    
  if ($tt_v2) {
    return x.toUpperCase();
  } }
  return "-";
}
export function ifAndElse(x: string | number, v: V) {
  let $tt_v5: boolean;
  let $tt_f_v5 = false;
  if (typeof x === "number") {
    let $tt_v3: boolean;
    {
      const $tt_m = v;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v3 = true;
          break;
        }
        case "B": {
          $tt_v3 = false;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v5 = $tt_v3;
    
  if ($tt_v5) { $tt_f_v5 = true; { return x.toFixed(1); } } } if ($tt_f_v5) {} else { return String(x); }
}
export function ifOr(y: string | null, v: V) {
  let $tt_v8: boolean;
  $tt_y_v8: {
    if (!(y === null)) {
      let $tt_v6: boolean;
      {
        const $tt_m = v;
        switch ($tt_m.kind) {
          case "A": {
            $tt_v6 = false;
            break;
          }
          case "B": {
            $tt_v6 = true;
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      $tt_v8 = $tt_v6;
      if (!($tt_v8)) break $tt_y_v8;
    }
    
  if (true) return 0; }
  return y.length;
}
export function loops(x: string | number, v: V) {
  let n = 0;
  while (true) {
    let $tt_v11: boolean;
    
    if (!(typeof x === "string")) break;
    let $tt_v9: boolean;
    {
      const $tt_m = v;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v9 = n < 2;
          break;
        }
        case "B": {
          $tt_v9 = false;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v11 = $tt_v9;
    if (!($tt_v11)) break;
    { L.push(x.toUpperCase()); n++; }}
  for (let i = 0; ; i++) {
    let $tt_v14: boolean;
    
    if (!(typeof x === "string")) break;
    let $tt_v12: boolean;
    {
      const $tt_m = v;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v12 = i < 1;
          break;
        }
        case "B": {
          $tt_v12 = false;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v14 = $tt_v12;
    if (!($tt_v14)) break;
    { L.push(x.toLowerCase()); }}
  let k = 0;
  while (true) {
    let $tt_v17: boolean;
    
    if (!(typeof x === "number")) {
      let $tt_v15: boolean;
      {
        const $tt_m = v;
        switch ($tt_m.kind) {
          case "A": {
            $tt_v15 = k++ < 1;
            break;
          }
          case "B": {
            $tt_v15 = false;
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      $tt_v17 = $tt_v15;
      if (!($tt_v17)) break;
    }
    { L.push(String(x)); if (typeof x === "number") break; }}
  return L.join();
}
console.log(ifAnd("ab", V.A), ifAnd("ab", V.B), ifAnd(1, V.A));
console.log(ifAndElse(2, V.A), ifAndElse(2, V.B), ifAndElse("s", V.A));
console.log(ifOr("abc", V.A), ifOr("abc", V.B), ifOr(null, V.A));
console.log(loops("Ab", V.A), loops(3, V.B));
