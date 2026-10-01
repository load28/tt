//// [semicolonFreeStatementsKeepTheirAutomaticBoundaries.tt] ////
variant O { A, B }
const log: string[] = []
const inc = (n: number) => n + 1
const note = (text: string) => { log.push(text) }
function run(x: O, y: O) {
log.push("start")
match (x) {
A => { log.push("xa") },
B => { log.push("xb") },
}
match (y) {
A => { log.push("ya") },
B => { log.push("yb") },
}
const n = 1
n |> inc |> String |> note
const piped = 1 |> inc
log.push(String(piped))
const next = 2
next |> String |> note
}
run(O.A, O.B)
console.log(log.join(","))

export {};


//// [semicolonFreeStatementsKeepTheirAutomaticBoundaries.ts]
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
type O =
  | { kind: "A" }
  | { kind: "B" };
const O = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const log: string[] = []
const inc = (n: number) => n + 1
const note = (text: string) => { log.push(text) }
function run(x: O, y: O) {
log.push("start")
let $tt_v0: undefined;
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "A": {
      log.push("xa")
        $tt_v0 = undefined;
        break;
    }
    case "B": {
      log.push("xb")
        $tt_v0 = undefined;
        break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}

let $tt_v1: undefined;
{
  const $tt_m = y;
  switch ($tt_m.kind) {
    case "A": {
      log.push("ya")
        $tt_v1 = undefined;
        break;
    }
    case "B": {
      log.push("yb")
        $tt_v1 = undefined;
        break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}

const n = 1
;(($tt_v, $tt_f) => $tt_f($tt_v))((($tt_v, $tt_f) => $tt_f($tt_v))((($tt_v, $tt_f) => $tt_f($tt_v))(n, inc), String), note)
const piped = inc(1)
log.push(String(piped))
const next = 2
;(($tt_v, $tt_f) => $tt_f($tt_v))((($tt_v, $tt_f) => $tt_f($tt_v))(next, String), note)
}
run(O.A, O.B)
console.log(log.join(","))

export {};
