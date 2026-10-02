//// [guardedContextualValuesPreserveShortCircuitingAndAbruptCompletion.tt] ////

const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
const receiver = {
  value: 10,
  consume(first: number, item: {kind: "item"; run: (x: number) => number}, last: number) {
    trace.push("call");
    return this.value + first + item.run(last);
  },
};
function guard(value: number): boolean {
  trace.push("guard");
  if (value === 2) throw new Error("guard failed");
  return value === 1;
}
for (const value of [0, 1, 2, 3]) {
  trace.length = 0;
  try {
    const result = mark("receiver", receiver).consume(mark("first", 2), match (mark("subject", value < 3)) {
      true if guard(value) => (trace.push("yes"), {kind: "item", run: x => x + 1}),
      true if mark("second guard", true) => (trace.push("second"), {kind: "item", run: x => x}),
      _ => (trace.push("no"), {kind: "item", run: x => x - 1}),
    }, mark("last", 3));
    console.log(result, trace.join(","));
  } catch {
    console.log("thrown", trace.join(","));
  }
}

export {};


//// [guardedContextualValuesPreserveShortCircuitingAndAbruptCompletion.ts]

const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
const receiver = {
  value: 10,
  consume(first: number, item: {kind: "item"; run: (x: number) => number}, last: number) {
    trace.push("call");
    return this.value + first + item.run(last);
  },
};
function guard(value: number): boolean {
  trace.push("guard");
  if (value === 2) throw new Error("guard failed");
  return value === 1;
}
for (const value of [0, 1, 2, 3]) {
  trace.length = 0;
  try {
    const $tt_v2 = (mark("receiver", receiver));
    const $tt_v3 = (mark("first", 2));
    const $tt_m = mark("subject", value < 3);
    
    const result = $tt_v2.consume($tt_v3, (($tt_m === true && guard(value)) ? (trace.push("yes"), {kind: "item", run: x => x + 1}) : ($tt_m === true && mark("second guard", true)) ? (trace.push("second"), {kind: "item", run: x => x}) : (trace.push("no"), {kind: "item", run: x => x - 1})), mark("last", 3));
    console.log(result, trace.join(","));
  } catch {
    console.log("thrown", trace.join(","));
  }
}

export {};
