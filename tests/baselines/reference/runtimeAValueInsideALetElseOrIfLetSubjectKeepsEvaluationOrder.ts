//// [runtimeAValueInsideALetElseOrIfLetSubjectKeepsEvaluationOrder.tt] ////

type Opt = { kind: "Some"; value: number } | { kind: "None" };
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const trace: string[] = [];
let pick = (n: number): Opt => { trace.push("pick" + n); return n > 1 ? { kind: "Some", value: n } : { kind: "None" }; };
function swap(n: number): number {
  trace.push("swap");
  const first = pick;
  pick = (m) => { trace.push("late" + m); return first(m); };
  return n;
}
function viaMatch(k: number): number {
  if let Some(value: w) = pick(match (k) { 1 => swap(1), _ => swap(3) }) { return w; }
  else if let Some(value: z) = pick(match (k) { 1 => swap(5), _ => 0 }) { return z * 10; }
  return -1;
}
function readOpt(n: number): R<Opt> {
  return n < 0 ? { kind: "Err", error: "neg" } : { kind: "Ok", value: n > 1 ? { kind: "Some", value: n } : { kind: "None" } };
}
function inResult(n: number) {
  return result {
    const Some(value: v) = (try readOpt(n)) else { return -2; };
    if let Some(value: q) = [try readOpt(n + 1)][0] { return v + q; }
    return v;
  };
}
console.log(viaMatch(1), JSON.stringify(trace));
console.log(JSON.stringify([inResult(2), inResult(1), inResult(-1)]));

export {};


//// [runtimeAValueInsideALetElseOrIfLetSubjectKeepsEvaluationOrder.ts]

type Opt = { kind: "Some"; value: number } | { kind: "None" };
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const trace: string[] = [];
let pick = (n: number): Opt => { trace.push("pick" + n); return n > 1 ? { kind: "Some", value: n } : { kind: "None" }; };
function swap(n: number): number {
  trace.push("swap");
  const first = pick;
  pick = (m) => { trace.push("late" + m); return first(m); };
  return n;
}
function viaMatch(k: number): number {
  {
    let $tt_t0; let $tt_v0: number;
    const $tt_v1: typeof pick = (pick);
    {
      const $tt_m = k;
      switch ($tt_m) {
        case 1: {
          $tt_v0 = swap(1);
          break;
        }
        default: {
          $tt_v0 = swap(3);
          break;
        }
      }
    }
    $tt_t0 = $tt_v1($tt_v0);
    if ($tt_t0.kind === "Some") {
      const { value: w } = $tt_t0;
      return w;
    } else {
      let $tt_t1; let $tt_v2: number;
      const $tt_v3: typeof pick = (pick);
      {
        const $tt_m = k;
        switch ($tt_m) {
          case 1: {
            $tt_v2 = swap(5);
            break;
          }
          default: {
            $tt_v2 = 0;
            break;
          }
        }
      }
      $tt_t1 = $tt_v3($tt_v2);
      if ($tt_t1.kind === "Some") {
        const { value: z } = $tt_t1;
        return z * 10;
      }
    }
  }
  return -1;
}
function readOpt(n: number): R<Opt> {
  return n < 0 ? { kind: "Err", error: "neg" } : { kind: "Ok", value: n > 1 ? { kind: "Some", value: n } : { kind: "None" } };
}
function inResult(n: number) {
  let $tt_v4: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v4: {
    let $tt_t2; let $tt_v5: Opt;
    const $tt_t3 = readOpt(n);
    if (!("value" in $tt_t3)) {
      $tt_v4 = $tt_t3;
      break $tt_v4;
    }
    $tt_v5 = $tt_t3.value;
    $tt_t2 = ($tt_v5);
    if ($tt_t2.kind !== "Some") {
      { const $tt_a0 = { value: { kind: "Ok" as const, value: -2 } }; $tt_v4 = $tt_a0.value; break $tt_v4; }
    }
    const { value: v } = $tt_t2;
    {
      let $tt_t4; let $tt_v6: Opt;
      const $tt_t5 = readOpt(n + 1);
      if (!("value" in $tt_t5)) {
        $tt_v4 = $tt_t5;
        break $tt_v4;
      }
      $tt_v6 = $tt_t5.value;
      $tt_t4 = [$tt_v6][0];
      if ($tt_t4.kind === "Some") {
        const { value: q } = $tt_t4;
        { const $tt_a1 = { value: { kind: "Ok" as const, value: v + q } }; $tt_v4 = $tt_a1.value; break $tt_v4; }
      }
    }
    {
      const $tt_a2 = { value: { kind: "Ok" as const, value: v } };
      $tt_v4 = $tt_a2.value;
      break $tt_v4;
    }
  }
  return $tt_v4;
}
console.log(viaMatch(1), JSON.stringify(trace));
console.log(JSON.stringify([inResult(2), inResult(1), inResult(-1)]));

export {};
