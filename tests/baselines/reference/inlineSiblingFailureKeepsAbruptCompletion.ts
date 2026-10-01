//// [inlineSiblingFailureKeepsAbruptCompletion.tt] ////

const trace: string[] = [];
function bad(): boolean { return JSON.parse('"bad"'); }
function pair(a: number, b: number) { trace.push("call"); }
try {
  pair(match (bad()) { true => 1, false => 2 }, match (true) { true => (trace.push("second"), 3), _ => 4 });
} catch (error) {
  console.log(error instanceof Error && error.message.includes("unexpected literal"), trace.length);
}

export {};


//// [inlineSiblingFailureKeepsAbruptCompletion.ts]
function $tt_raise(error: unknown): never { throw error; }
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
function bad(): boolean { return JSON.parse('"bad"'); }
function pair(a: number, b: number) { trace.push("call"); }
try {
  let $tt_subject;
  let $tt_subject_1;
  
  pair(($tt_subject = bad(), ($tt_subject === true) ? 1 : ($tt_subject === false) ? 2 : $tt_raise(new Error("tt match: unexpected literal " + $tt_show($tt_subject)))), ($tt_subject_1 = true, ($tt_subject_1 === true) ? (trace.push("second"), 3) : 4));
} catch (error) {
  console.log(error instanceof Error && error.message.includes("unexpected literal"), trace.length);
}

export {};
