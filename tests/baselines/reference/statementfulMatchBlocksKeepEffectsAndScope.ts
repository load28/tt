//// [statementfulMatchBlocksKeepEffectsAndScope.tt] ////

const trace: string[] = [];
const label = 9;
function consume(value: number) { trace.push("call"); return value; }
for (const flag of [true, false]) {
  trace.length = 0;
  const result = consume(match (flag) {
    true => { const label = 3; trace.push("before"); return label; },
    false => { try { return label; } finally { trace.push("finally"); } },
  });
  console.log(result, label, trace.join(","));
}

export {};


//// [statementfulMatchBlocksKeepEffectsAndScope.ts]
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

const trace: string[] = [];
const label = 9;
function consume(value: number) { trace.push("call"); return value; }
for (const flag of [true, false]) {
  trace.length = 0;
  let $tt_v0: number;
  const $tt_v1: typeof consume = (consume);
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        const label = 3; trace.push("before"); $tt_v0 = label; break;
      }
      case false: {
        try { $tt_v0 = label; break; } finally { trace.push("finally"); }
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  const result = $tt_v1($tt_v0);
  console.log(result, label, trace.join(","));
}

export {};
