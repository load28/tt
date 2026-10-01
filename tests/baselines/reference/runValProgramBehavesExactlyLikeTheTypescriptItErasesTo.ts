//// [runValProgramBehavesExactlyLikeTheTypescriptItErasesTo.tt] ////

val const config = { name: "tt", tags: ["dev"] };
val let state = { count: 0 };

function describe(val c: { name: string; tags: string[] }): string {
  return `${c.name}:${c.tags.length}`;
}

function bump(s: { count: number }) {
  s.count += 1;
  return s;
}

state = { count: state.count + 1 };
const mutable = { count: 0 };
bump(mutable);

console.log(describe(config));
console.log(String(state.count));
console.log(String(mutable.count));

export {};


//// [runValProgramBehavesExactlyLikeTheTypescriptItErasesTo.ts]

const config = { name: "tt", tags: ["dev"] };
let state = { count: 0 };

function describe(c: { name: string; tags: string[] }): string {
  return `${c.name}:${c.tags.length}`;
}

function bump(s: { count: number }) {
  s.count += 1;
  return s;
}

state = { count: state.count + 1 };
const mutable = { count: 0 };
bump(mutable);

console.log(describe(config));
console.log(String(state.count));
console.log(String(mutable.count));

export {};
