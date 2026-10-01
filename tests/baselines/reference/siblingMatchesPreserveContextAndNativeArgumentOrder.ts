//// [siblingMatchesPreserveContextAndNativeArgumentOrder.tt] ////

const trace: string[] = [];
type Item = {kind: "item"; run: (x: number) => number};
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
let flag: boolean = true;
const receiver = {
  base: 10,
  get pair() {
    trace.push("callee");
    return function(this: {base: number}, a: Item, between: number, b: Item) {
      trace.push("call"); return this.base + a.run(between) + b.run(2);
    };
  },
};
const result = mark("receiver", receiver).pair(
  match (mark("subject1", flag)) {
    true => (trace.push("arm1"), flag = false, {kind: "item", run: x => x + 1}),
    false => ({kind: "item", run: x => x}),
  },
  mark("between", 3),
  match (mark("subject2", flag)) {
    true => ({kind: "item", run: x => x}),
    false => { return (trace.push("arm2"), {kind: "item", run: x => x + 2}); },
  },
);
console.log(result, trace.join(","));

export {};


//// [siblingMatchesPreserveContextAndNativeArgumentOrder.ts]
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
type Item = {kind: "item"; run: (x: number) => number};
function mark<T>(name: string, value: T): T { trace.push(name); return value; }
let flag: boolean = true;
const receiver = {
  base: 10,
  get pair() {
    trace.push("callee");
    return function(this: {base: number}, a: Item, between: number, b: Item) {
      trace.push("call"); return this.base + a.run(between) + b.run(2);
    };
  },
};
let $tt_subject;
let $tt_subject_1;

const result = mark("receiver", receiver).pair(
  ($tt_subject = mark("subject1", flag), ($tt_subject === true) ? (trace.push("arm1"), flag = false, {kind: "item", run: x => x + 1}) : ($tt_subject === false) ? ({kind: "item", run: x => x}) : $tt_raise(new Error("tt match: unexpected literal " + $tt_show($tt_subject)))),
  mark("between", 3),
  ($tt_subject_1 = mark("subject2", flag), ($tt_subject_1 === true) ? ({kind: "item", run: x => x}) : ($tt_subject_1 === false) ? (trace.push("arm2"), {kind: "item", run: x => x + 2}) : $tt_raise(new Error("tt match: unexpected literal " + $tt_show($tt_subject_1)))),
);
console.log(result, trace.join(","));

export {};
