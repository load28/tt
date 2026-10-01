//// [deferredMatchValuesPreserveOrderAndReceiver.tt] ////

const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
const receiver = {
  value: 10,
  consume(first: number, item: {kind: "item"; run: (x: number) => number}, last: number) {
    trace.push("call");
    return this.value + first + item.run(last);
  },
};
for (const flag of [true, false]) {
  trace.length = 0;
  const value = mark("receiver", receiver).consume(mark("first", 2), match (mark("subject", flag)) {
    true => (trace.push("yes"), {kind: "item", run: x => x + 1}),
    _ => (trace.push("no"), {kind: "item", run: x => x - 1}),
  }, mark("last", 3));
  console.log(value, trace.join(","));
}

export {};


//// [deferredMatchValuesPreserveOrderAndReceiver.ts]

const trace: string[] = [];
const mark = <T,>(name: string, value: T): T => { trace.push(name); return value; };
const receiver = {
  value: 10,
  consume(first: number, item: {kind: "item"; run: (x: number) => number}, last: number) {
    trace.push("call");
    return this.value + first + item.run(last);
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
      default: $tt_v0 = 1; break;
    }
  }
  const value = $tt_v2.consume($tt_v3, ($tt_v0 === 0 ? (trace.push("yes"), {kind: "item", run: x => x + 1}) : (trace.push("no"), {kind: "item", run: x => x - 1})), mark("last", 3));
  console.log(value, trace.join(","));
}

export {};
