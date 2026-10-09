//// [templateSubstitutionsBeforeALoweredValueRunFirstInArmsAndGuards.tt] ////
variant S { A, B }
const log: string[] = [];
function o(t: string): S { log.push("subject" + t); return S.A; }
const g = { get p() { log.push("g.p"); return 1; } };
function F(x: unknown) { return x; }
function main() {
  const a = match (o("0")) { A if `${g.p}${match (o("1")) { A => 1, B => 2 }}` !== "" => 1, _ => 2 };
  log.push("|");
  const b = match (o("0")) { A => { return `${g.p}${match (o("1")) { A => 1, B => 2 }}`; }, B => "" };
  log.push("|");
  const c = match (o("0")) { A => [g.p, match (o("1")) { A => 1, B => 2 }], B => [] };
  log.push("|");
  const d = match (o("0")) { A => g.p + match (o("1")) { A => 1, B => 2 }, B => 0 };
  log.push("|");
  const e = match (o("0")) { A => ({ x: g.p, y: match (o("1")) { A => 1, B => 2 } }), B => 0 };
  log.push("|");
  const h = 1 |> F |> (v => `${g.p}${match (o("1")) { A => 1, B => 2 }}`);
}
main();
console.log(log.join(", "));


//// [templateSubstitutionsBeforeALoweredValueRunFirstInArmsAndGuards.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
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
const g = { get p() { log.push("g.p"); return 1; } };
function F(x: unknown) { return x; }
function main() {
  let $tt_v0: number;
  {
    const $tt_m = o("0");
    do {
      if ($tt_m.kind === "A") {
        let $tt_v1: number;
        const $tt_v2 = (`${g.p}`);
        {
          const $tt_m = o("1");
          switch ($tt_m.kind) {
            case "A": $tt_v1 = 0; break;
            case "B": $tt_v1 = 1; break;
            default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
        if (`${$tt_v2}${($tt_v1 === 0 ? 1 : 2)}` !== "") {
          $tt_v0 = 1;
          break;
        }
      }
      $tt_v0 = 2;
      break;
    } while (false);
  }
  const a = $tt_v0;
  log.push("|");
  let $tt_v3: string;
  {
    const $tt_m = o("0");
    switch ($tt_m.kind) {
      case "A": {
        const $tt_v = (`${g.p}`);
        let $tt_v15: number;
        {
          const $tt_m = o("1");
          switch ($tt_m.kind) {
            case "A": {
              $tt_v15 = 1;
              break;
            }
            case "B": {
              $tt_v15 = 2;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        $tt_v3 = `${$tt_v}${$tt_v15}`;
    break;
      }
      case "B": {
        $tt_v3 = "";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const b = $tt_v3;
  log.push("|");
  let $tt_v4: number[];
  {
    const $tt_m = o("0");
    switch ($tt_m.kind) {
      case "A": {
        let $tt_v5: number;
        const $tt_v6: typeof g.p = (g.p);
        {
          const $tt_m = o("1");
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
        const $tt_a0 = { value: [$tt_v6, $tt_v5] };
        $tt_v4 = $tt_a0.value;
        break;
      }
      case "B": {
        const $tt_a1 = { value: [] };
        $tt_v4 = $tt_a1.value;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const c = $tt_v4;
  log.push("|");
  let $tt_v7: number;
  {
    const $tt_m = o("0");
    switch ($tt_m.kind) {
      case "A": {
        let $tt_v8: number;
        const $tt_v9: typeof g.p = (g.p);
        {
          const $tt_m = o("1");
          switch ($tt_m.kind) {
            case "A": {
              $tt_v8 = 1;
              break;
            }
            case "B": {
              $tt_v8 = 2;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        $tt_v7 = $tt_v9 + $tt_v8;
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
  const d = $tt_v7;
  log.push("|");
  let $tt_v10: ({
    x: number;
    y: number;
}) | (number);
  {
    const $tt_m = o("0");
    switch ($tt_m.kind) {
      case "A": {
        let $tt_v11: number;
        const $tt_v12: typeof g.p = (g.p);
        {
          const $tt_m = o("1");
          switch ($tt_m.kind) {
            case "A": {
              $tt_v11 = 1;
              break;
            }
            case "B": {
              $tt_v11 = 2;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        $tt_v10 = ({ x: $tt_v12, y: $tt_v11 });
        break;
      }
      case "B": {
        $tt_v10 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const e = $tt_v10;
  log.push("|");
  const h = $tt_ap(F(1), ((v => {
    let $tt_v13: number;
    const $tt_v14 = (`${g.p}`);
    {
      const $tt_m = o("1");
      switch ($tt_m.kind) {
        case "A": $tt_v13 = 0; break;
        case "B": $tt_v13 = 1; break;
        default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
    return `${$tt_v14}${($tt_v13 === 0 ? 1 : 2)}`;
  })));
}
main();
console.log(log.join(", "));
