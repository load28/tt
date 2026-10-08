//// [aDiscardedCommaOperandInAConditionalBranchIsNotAValue.tt] ////
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function r(): R { return { kind: "Ok", value: 7 }; }
const c = true;
variant S { A(n: number), B }
const s: S = S.A(3) as S;
function f(): R {
  const a = c ? (try r(), 1) : 2;
  const b = c && (match (s) { A(n) => n, B => 0 }, 1);
  const i = !c || (try r(), 4);
  const z = (try r(), 5);
  return { kind: "Ok", value: a + +b + +i + z };
}
console.log(JSON.stringify(f()));


//// [aDiscardedCommaOperandInAConditionalBranchIsNotAValue.ts]
var $tt_show: (value: unknown) => string = function (value) {
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
};
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function r(): R { return { kind: "Ok", value: 7 }; }
const c = true;
type S =
  | { kind: "A"; n: number }
  | { kind: "B" };
const S = {
  A: (n: number): S => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const s: S = S.A(3) as S;
function f(): R {
  let $tt_v2: number;
  if (c) {
    let $tt_v0: number;
    const $tt_t0 = r();
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v0 = $tt_t0.value;
    $tt_v2 =  1;
  } else {
    $tt_v2 = 2;
  }
  
  const a = $tt_v2;
  let $tt_v5: number;
  let $tt_v4: true;
  if ($tt_v4 = c) {
    let $tt_v3: number;
    {
      const $tt_m = s;
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
    $tt_v5 = $tt_v4 &&  1;
  } else {
    $tt_v5 = $tt_v4;
  }
  
  const b = $tt_v5;
  let $tt_v8: number;
  let $tt_v7: false;
  if ($tt_v7 = !c) {
    $tt_v8 = $tt_v7;
  } else {
    let $tt_v6: number;
    const $tt_t1 = r();
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    $tt_v6 = $tt_t1.value;
    $tt_v8 = $tt_v7 ||  4;
  }
  
  const i = $tt_v8;
  let $tt_v9: number;
  const $tt_t2 = r();
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  $tt_v9 = $tt_t2.value;
  const z = ( 5);
  return { kind: "Ok", value: a + +b + +i + z };
}
console.log(JSON.stringify(f()));
