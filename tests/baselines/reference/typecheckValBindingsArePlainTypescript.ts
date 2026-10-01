//// [typecheckValBindingsArePlainTypescript.tt] ////

type User = { name: string; tags: string[] };

val const user: User = { name: "Kim", tags: ["dev"] };

function inspect(val u: User): string {
  return u.name + u.tags.length;
}

val let state = { count: 0 };
state = { ...state, count: state.count + 1 };

const label = inspect(user) + state.count;

export {};


//// [typecheckValBindingsArePlainTypescript.ts]

type User = { name: string; tags: string[] };

const user: User = { name: "Kim", tags: ["dev"] };

function inspect(u: User): string {
  return u.name + u.tags.length;
}

let state = { count: 0 };
state = { ...state, count: state.count + 1 };

const label = inspect(user) + state.count;

export {};
