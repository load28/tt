//// [runtimeAValueInsideARegionReturnArgumentRunsInTheReturnSPrelude.tt] ////

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
function ok(n: number): R { return { kind: "Ok", value: n }; }
let s = (n: number): string => { trace.push("s" + n); return "s" + n; };
function swap(n: number): number {
  trace.push("swap");
  const first = s;
  s = (m) => { trace.push("late"); return first(m); };
  return n;
}
function inResult(v: number) {
  return result {
    const q = try ok(1);
    if (v > 5) return s(match (v) { 6 => swap(6), _ => 2 }) + q;
    return [match (v) { 1 => "a", _ => "b" }, match (q) { 1 => "c", _ => "d" }].join("") + q;
  };
}
function inArm(v: number) {
  const x = match (v) {
    1 => { const q = 1; if (q > 0) return s(match (v) { 1 => swap(1), _ => 2 }) + q; return "n"; },
    2 => { return `${match (v) { 2 => "t", _ => "u" }}-`; },
    _ => "x",
  };
  return x;
}
function template(v: number) {
  return result { const q = try ok(1); return `${match (v) { 1 => "a", _ => "b" }}-${q}`; };
}
const base = s;
console.log(JSON.stringify([inResult(6), inResult(1)]), JSON.stringify(trace));
s = base;
trace.length = 0;
console.log(JSON.stringify([inArm(1), inArm(2), inArm(3)]), JSON.stringify(trace));
console.log(JSON.stringify([template(1), template(2)]));

export {};


//// [runtimeAValueInsideARegionReturnArgumentRunsInTheReturnSPrelude.ts]

type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
function ok(n: number): R { return { kind: "Ok", value: n }; }
let s = (n: number): string => { trace.push("s" + n); return "s" + n; };
function swap(n: number): number {
  trace.push("swap");
  const first = s;
  s = (m) => { trace.push("late"); return first(m); };
  return n;
}
function inResult(v: number) {
  let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: string;
});
  $tt_v0: {
    const $tt_t0 = ok(1);
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const q = $tt_t0.value;
    if (v > 5) {
      let $tt_v1: number;
      const $tt_v2: typeof s = (s);
      {
        const $tt_m = v;
        switch ($tt_m) {
          case 6: $tt_v1 = 0; break;
          default: $tt_v1 = 1; break;
        }
      }
      { const $tt_a0 = { value: { kind: "Ok" as const, value: $tt_v2(($tt_v1 === 0 ? swap(6) : 2)) + q } }; $tt_v0 = $tt_a0.value; break $tt_v0; }
    }
    let $tt_subject_1;
    let $tt_subject_2;
    
    {
      const $tt_a1 = { value: { kind: "Ok" as const, value: [($tt_subject_1 = v, ($tt_subject_1 === 1) ? "a" : "b"), ($tt_subject_2 = q, ($tt_subject_2 === 1) ? "c" : "d")].join("") + q } };
      $tt_v0 = $tt_a1.value;
      break $tt_v0;
    }
  }
  return $tt_v0;
}
function inArm(v: number) {
  let $tt_v5: string;
  {
    const $tt_m = v;
    switch ($tt_m) {
      case 1: {
        const q = 1; if (q > 0) {
          let $tt_v6: number;
          const $tt_v7: typeof s = (s);
          {
            const $tt_m = v;
            switch ($tt_m) {
              case 1: $tt_v6 = 0; break;
              default: $tt_v6 = 1; break;
            }
          }
          { $tt_v5 = $tt_v7(($tt_v6 === 0 ? swap(1) : 2)) + q; break; }
        } $tt_v5 = "n"; break;
      }
      case 2: {
        let $tt_v9: string;
        {
          const $tt_m = v;
          switch ($tt_m) {
            case 2: {
              $tt_v9 = "t";
              break;
            }
            default: {
              $tt_v9 = "u";
              break;
            }
          }
        }
        $tt_v5 = `${$tt_v9}-`;
    break;
      }
      default: {
        $tt_v5 = "x";
        break;
      }
    }
  }
  const x = $tt_v5;
  return x;
}
function template(v: number) {
  let $tt_v8: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: string;
});
  $tt_v8: {
    const $tt_t1 = ok(1);
    if (!("value" in $tt_t1)) {
      $tt_v8 = $tt_t1;
      break $tt_v8;
    }
    const q = $tt_t1.value; let $tt_v10: string;
    {
      const $tt_m = v;
      switch ($tt_m) {
        case 1: {
          $tt_v10 = "a";
          break;
        }
        default: {
          $tt_v10 = "b";
          break;
        }
      }
    }
    const $tt_a2 = { value: { kind: "Ok" as const, value: `${$tt_v10}-${q}` } };
    $tt_v8 = $tt_a2.value;
    break $tt_v8;
  }
  return $tt_v8;
}
const base = s;
console.log(JSON.stringify([inResult(6), inResult(1)]), JSON.stringify(trace));
s = base;
trace.length = 0;
console.log(JSON.stringify([inArm(1), inArm(2), inArm(3)]), JSON.stringify(trace));
console.log(JSON.stringify([template(1), template(2)]));

export {};
