//// [singleReturnMatchBlocksKeepContextAndRuntimeOrder.tt] ////

const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
const receiver = {
  base: 10,
  consume(first: number, item: {kind: "item"; run: (x: number) => number}, last: number) {
    trace.push("call");
    return this.base + first + item.run(last);
  },
};
for (const flag of [true, false]) {
  trace.length = 0;
  const answer = mark("receiver", receiver).consume(mark("first", 2), match (mark("subject", flag)) {
    true => { /* selected block */ return (mark("yes", 0), {kind: "item", run: x => x + 1}); /* tail */ },
    false => { return (mark("no", 0), {kind: "item", run: x => x - 1}); },
  }, mark("last", 3));
  console.log(answer, trace.join(","));
}

export {};


//// [singleReturnMatchBlocksKeepContextAndRuntimeOrder.ts]
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
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
const receiver = {
  base: 10,
  consume(first: number, item: {kind: "item"; run: (x: number) => number}, last: number) {
    trace.push("call");
    return this.base + first + item.run(last);
  },
};
for (const flag of [true, false]) {
  trace.length = 0;
  let $tt_v0: number;
  const $tt_v2 = (mark("receiver", receiver));
  const $tt_v3 = (mark("first", 2));
  {
    const $tt_m = mark("subject", flag);
    switch ($tt_m) {
      case true: $tt_v0 = 0; break;
      case false: $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
  const answer = $tt_v2.consume($tt_v3, ($tt_v0 === 0 ? /* selected block */  (mark("yes", 0), {kind: "item", run: x => x + 1}) /* tail */ : (mark("no", 0), {kind: "item", run: x => x - 1})), mark("last", 3));
  console.log(answer, trace.join(","));
}

export {};
