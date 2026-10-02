//// [impossibleCaseAtPattern.tt] ////
// Repro from TASK-620
variant S { A, B }
declare const s: S;
export const r = match (s) { A => 1, Zzz => 2 };
match (s) { A => {}, B => {}, Yyy | Www => {} }


//// [impossibleCaseAtPattern.ts]
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
// Repro from TASK-620
type S =
  | { kind: "A" }
  | { kind: "B" };
const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
declare const s: S;
let $tt_v0: number;
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "A": {
      $tt_v0 = 1;
      break;
    }
    case "Zzz": {
      $tt_v0 = 2;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
export const r = $tt_v0;
let $tt_v1: undefined;
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "A": {
      
        $tt_v1 = undefined;
        break;
    }
    case "B": {
      
        $tt_v1 = undefined;
        break;
    }
    case "Yyy": case "Www": {
      
        $tt_v1 = undefined;
        break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}

