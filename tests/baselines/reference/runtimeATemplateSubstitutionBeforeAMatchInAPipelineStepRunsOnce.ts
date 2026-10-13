//// [runtimeATemplateSubstitutionBeforeAMatchInAPipelineStepRunsOnce.tt] ////

variant S { A, B }
const log: string[] = [];
function head(): string {
  log.push("head");
  return "H";
}
function wrap(text: string): (input: string) => string {
  log.push("wrap");
  return (input) => `${input}:${text}`;
}
function run(s: S): string {
  return "in" |> wrap(`${head()}${match (s) { A => (log.push("A"), 1), B => 2 }}`);
}
console.log(run(S.A), log.join(","));
log.length = 0;
console.log(run(S.B), log.join(","));

export {};


//// [runtimeATemplateSubstitutionBeforeAMatchInAPipelineStepRunsOnce.ts]
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

type S =
  | { kind: "A" }
  | { kind: "B" };
const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const log: string[] = [];
function head(): string {
  log.push("head");
  return "H";
}
function wrap(text: string): (input: string) => string {
  log.push("wrap");
  return (input) => `${input}:${text}`;
}
function run(s: S): string {
  let $tt_v0: string;
  do {
    const $tt_v4 = "in";
    let $tt_v1: number;
    const $tt_v3: typeof wrap = (wrap);
    const $tt_v2 = (`${head()}`);
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v1 = (log.push("A"), 1);
          break;
        }
        case "B": {
          $tt_v1 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v0 = $tt_v3(`${$tt_v2}${$tt_v1}`)($tt_v4);
    break;
  } while (false);
  return $tt_v0;
}
console.log(run(S.A), log.join(","));
log.length = 0;
console.log(run(S.B), log.join(","));

export {};
