//// [aPatternSubjectWithAPlainTemplateAfterAMatchIsLowered.tt] ////
variant V { A(n: number), B }
const pick = (k: number, label: string): V => (label.length > 3 ? V.A(k) : V.B);
function viaLetElse(k: number) {
  const A(n) = pick(match (k) { 1 => 10, _ => 20 }, `k=${k}!`) else { return -1; };
  return n;
}
function viaIfLet(k: number) {
  if let A(n) = pick(match (k) { 1 => 10, _ => 20 }, `${k}`) {
    return n;
  }
  return -2;
}
function viaOr(k: number, s: string) {
  const A(n) = pick(match (k) { 1 => 10, _ => 20 }, s || `k=${k}!`) else { return -3; };
  return n;
}
console.log(viaLetElse(1), viaLetElse(2), viaIfLet(1), viaOr(5, ""));


//// [aPatternSubjectWithAPlainTemplateAfterAMatchIsLowered.ts]
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const pick = (k: number, label: string): V => (label.length > 3 ? V.A(k) : V.B);
function viaLetElse(k: number) {
  let $tt_t0; let $tt_v0: number;
  const $tt_v1: typeof pick = (pick);
  {
    const $tt_m = k;
    switch ($tt_m) {
      case 1: {
        $tt_v0 = 10;
        break;
      }
      default: {
        $tt_v0 = 20;
        break;
      }
    }
  }
  $tt_t0 = $tt_v1($tt_v0, `k=${k}!`);
  if ($tt_t0.kind !== "A") {
    return -1;
  }
  const { n } = $tt_t0;
  return n;
}
function viaIfLet(k: number) {
  {
    let $tt_t1; let $tt_v2: number;
    const $tt_v3: typeof pick = (pick);
    {
      const $tt_m = k;
      switch ($tt_m) {
        case 1: {
          $tt_v2 = 10;
          break;
        }
        default: {
          $tt_v2 = 20;
          break;
        }
      }
    }
    $tt_t1 = $tt_v3($tt_v2, `${k}`);
    if ($tt_t1.kind === "A") {
      const { n } = $tt_t1;
      return n;
    }
  }
  return -2;
}
function viaOr(k: number, s: string) {
  let $tt_t2; let $tt_v4: number;
  const $tt_v5: typeof pick = (pick);
  {
    const $tt_m = k;
    switch ($tt_m) {
      case 1: {
        $tt_v4 = 10;
        break;
      }
      default: {
        $tt_v4 = 20;
        break;
      }
    }
  }
  $tt_t2 = $tt_v5($tt_v4, s || `k=${k}!`);
  if ($tt_t2.kind !== "A") {
    return -3;
  }
  const { n } = $tt_t2;
  return n;
}
console.log(viaLetElse(1), viaLetElse(2), viaIfLet(1), viaOr(5, ""));
