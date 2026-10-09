//// [anOptionalCallArgumentRunsWholeBeforeALaterValue.tt] ////
variant S { A, B }
const log: string[] = [];
function o(t: string): S { log.push("subject" + t); return S.A; }
function L(s: string) { log.push(s); return s; }
function F(s: unknown) { log.push("F"); return s; }
const g = { mm: (...a: unknown[]) => a.length };
function f(...a: unknown[]) { log.push("f"); return a.length; }
function main() {
  g.mm?.(f(match (o("1")) { A => 1, B => 2 } |> F, L("x")), match (o("2")) { A => 1, B => 2 });
  log.push("|");
  g.mm(f(match (o("1")) { A => 1, B => 2 } |> F, L("x")), match (o("2")) { A => 1, B => 2 });
  log.push("|");
  f(f(match (o("1")) { A => 1, B => 2 } |> F, L("x")), match (o("2")) { A => 1, B => 2 });
  log.push("|");
  f(f(1 |> F, L("x")), match (o("2")) { A => 1, B => 2 });
  log.push("|");
  f([match (o("1")) { A => 1, B => 2 } |> F, L("x")], match (o("2")) { A => 1, B => 2 });
  log.push("|");
  const r = [f(match (o("1")) { A => 1, B => 2 } |> F, L("x")), match (o("2")) { A => 1, B => 2 }];
}
main();
console.log(log.join(", "));


//// [anOptionalCallArgumentRunsWholeBeforeALaterValue.ts]
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
type S =
  | { kind: "A" }
  | { kind: "B" };
const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const log: string[] = [];
function o(t: string): S { log.push("subject" + t); return S.A; }
function L(s: string) { log.push(s); return s; }
function F(s: unknown) { log.push("F"); return s; }
const g = { mm: (...a: unknown[]) => a.length };
function f(...a: unknown[]) { log.push("f"); return a.length; }
function main() {
  let $tt_v7: (number) | (undefined);
  const $tt_v4 = (g.mm);
  if ($tt_v4 != null) {
    let $tt_v0;
    const $tt_v3: typeof f = (f);
    do {
      let $tt_v35: number;
      {
        const $tt_m = o("1");
        switch ($tt_m.kind) {
          case "A": {
            $tt_v35 = 1;
            break;
          }
          case "B": {
            $tt_v35 = 2;
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      $tt_v0 = F($tt_v35);
      break;
    } while (false);
    const $tt_v6: number = ($tt_v3($tt_v0, L("x")));
    let $tt_v2: number;
    {
      const $tt_m = o("2");
      switch ($tt_m.kind) {
        case "A": {
          $tt_v2 = 1;
          break;
        }
        case "B": {
          $tt_v2 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v7 = $tt_v4.call(g, $tt_v6, $tt_v2);
  } else {
    $tt_v7 = undefined;
  }
  
  $tt_v7;
  log.push("|");
  let $tt_v8;
  const $tt_v11: typeof f = (f);
  do {
    let $tt_v36: number;
    {
      const $tt_m = o("1");
      switch ($tt_m.kind) {
        case "A": {
          $tt_v36 = 1;
          break;
        }
        case "B": {
          $tt_v36 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v8 = F($tt_v36);
    break;
  } while (false);
  const $tt_v13 = ($tt_v11($tt_v8, L("x")));
  {
    const $tt_m = o("2");
    switch ($tt_m.kind) {
      case "A": {
        g.mm($tt_v13, 1);
        break;
      }
      case "B": {
        g.mm($tt_v13, 2);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
  log.push("|");
  let $tt_v14;
  const $tt_v18: typeof f = (f);
  const $tt_v17: typeof f = (f);
  do {
    let $tt_v37: number;
    {
      const $tt_m = o("1");
      switch ($tt_m.kind) {
        case "A": {
          $tt_v37 = 1;
          break;
        }
        case "B": {
          $tt_v37 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v14 = F($tt_v37);
    break;
  } while (false);
  const $tt_v19 = ($tt_v17($tt_v14, L("x")));
  {
    const $tt_m = o("2");
    switch ($tt_m.kind) {
      case "A": {
        $tt_v18($tt_v19, 1);
        break;
      }
      case "B": {
        $tt_v18($tt_v19, 2);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
  log.push("|");
  const $tt_v23: typeof f = (f);
  const $tt_v24 = (f(F(1), L("x")));
  {
    const $tt_m = o("2");
    switch ($tt_m.kind) {
      case "A": {
        $tt_v23($tt_v24, 1);
        break;
      }
      case "B": {
        $tt_v23($tt_v24, 2);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
  log.push("|");
  let $tt_v25;
  const $tt_v28: typeof f = (f);
  do {
    let $tt_v38: number;
    {
      const $tt_m = o("1");
      switch ($tt_m.kind) {
        case "A": {
          $tt_v38 = 1;
          break;
        }
        case "B": {
          $tt_v38 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v25 = F($tt_v38);
    break;
  } while (false);
  const $tt_v29 = ([$tt_v25, L("x")]);
  {
    const $tt_m = o("2");
    switch ($tt_m.kind) {
      case "A": {
        $tt_v28($tt_v29, 1);
        break;
      }
      case "B": {
        $tt_v28($tt_v29, 2);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  
  log.push("|");
  let $tt_v30;
  let $tt_v32: number;
  const $tt_v33: typeof f = (f);
  do {
    let $tt_v39: number;
    {
      const $tt_m = o("1");
      switch ($tt_m.kind) {
        case "A": {
          $tt_v39 = 1;
          break;
        }
        case "B": {
          $tt_v39 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v30 = F($tt_v39);
    break;
  } while (false);
  const $tt_v34 = ($tt_v33($tt_v30, L("x")));
  {
    const $tt_m = o("2");
    switch ($tt_m.kind) {
      case "A": {
        $tt_v32 = 1;
        break;
      }
      case "B": {
        $tt_v32 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const r = [$tt_v34, $tt_v32];
}
main();
console.log(log.join(", "));
