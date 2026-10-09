//// [templateSubstitutionsBeforeALoweredValueRunFirst.tt] ////
variant S { A, B }
const log: string[] = [];
function o(): S { log.push("subject"); return S.A; }
const g = { get p() { log.push("g.p"); return 1; } };
class K { constructor() { log.push("new K"); } toString() { return "K"; } }
function tag(s: TemplateStringsArray, ...v: unknown[]) { return v.join(","); }
function f(...a: unknown[]) { return a.join(","); }
function main() {
  const a = match (o()) { A => `a${g.p && 2}b${match (o()) { A => 1, B => 2 }}`, B => "" };
  log.push("| " + a);
  const b = match (o()) { A => tag`q${new K()}r${match (o()) { A => 1, B => 2 }}`, B => "" };
  log.push("| " + b);
  const c = match (o()) { A => f(g.p && 2, match (o()) { A => 1, B => 2 }), B => "" };
  log.push("| " + c);
  const d = match (o()) { A => `a${g.p}b${match (o()) { A => 1, B => 2 }}`, B => "" };
  log.push("| " + d);
  const e = f(`a${g.p && 2}b${match (o()) { A => 1, B => 2 }}`);
  log.push("| " + e);
}
main();
console.log(log.join(", "));


//// [templateSubstitutionsBeforeALoweredValueRunFirst.ts]
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
function o(): S { log.push("subject"); return S.A; }
const g = { get p() { log.push("g.p"); return 1; } };
class K { constructor() { log.push("new K"); } toString() { return "K"; } }
function tag(s: TemplateStringsArray, ...v: unknown[]) { return v.join(","); }
function f(...a: unknown[]) { return a.join(","); }
function main() {
  let $tt_v0: string;
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": {
        const $tt_v = (`${g.p && 2}`);
        let $tt_v10: number;
        {
          const $tt_m = o();
          switch ($tt_m.kind) {
            case "A": {
              $tt_v10 = 1;
              break;
            }
            case "B": {
              $tt_v10 = 2;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        $tt_v0 = `a${$tt_v}b${$tt_v10}`;
        break;
      }
      case "B": {
        $tt_v0 = "";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const a = $tt_v0;
  log.push("| " + a);
  let $tt_v1: string;
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": {
        const $tt_v = (new K());
        let $tt_v11: number;
        {
          const $tt_m = o();
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
        $tt_v1 = tag`q${$tt_v}r${$tt_v11}`;
        break;
      }
      case "B": {
        $tt_v1 = "";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const b = $tt_v1;
  log.push("| " + b);
  let $tt_v2: string;
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": {
        let $tt_v3: number;
        const $tt_v4: typeof f = (f);
        const $tt_v5 = (g.p && 2);
        {
          const $tt_m = o();
          switch ($tt_m.kind) {
            case "A": {
              $tt_v3 = 1;
              break;
            }
            case "B": {
              $tt_v3 = 2;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        $tt_v2 = $tt_v4($tt_v5, $tt_v3);
        break;
      }
      case "B": {
        $tt_v2 = "";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const c = $tt_v2;
  log.push("| " + c);
  let $tt_v6: string;
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": {
        const $tt_v = (`${g.p}`);
        let $tt_v12: number;
        {
          const $tt_m = o();
          switch ($tt_m.kind) {
            case "A": {
              $tt_v12 = 1;
              break;
            }
            case "B": {
              $tt_v12 = 2;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        $tt_v6 = `a${$tt_v}b${$tt_v12}`;
        break;
      }
      case "B": {
        $tt_v6 = "";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const d = $tt_v6;
  log.push("| " + d);
  let $tt_v7: number;
  const $tt_v9: typeof f = (f);
  const $tt_v8 = (`${g.p && 2}`);
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": $tt_v7 = 0; break;
      case "B": $tt_v7 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  const e = $tt_v9(`a${$tt_v8}b${($tt_v7 === 0 ? 1 : 2)}`);
  log.push("| " + e);
}
main();
console.log(log.join(", "));
