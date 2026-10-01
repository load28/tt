//// [runtimeAnIfLetAsAnUnbracedBodyKeepsItsParentAndItsElse.tt] ////

variant O { Some(value: number), None }
function f(xs: O[]): number {
  let t = 0;
  for (const x of xs) if let Some(value) = x { t += value; } else { break; }
  return t;
}
function g(c: boolean, x: O): number {
  if (c) if let Some(value) = x { return value; } else { return 2; }
  else { return 3; }
}
function h(c: boolean, x: O): number {
  if (c) if let Some(value) = x { return value; }
  else { return 3; }
  return 4;
}
function k(xs: O[]): number {
  let t = 0;
  outer: for (const x of xs) if let Some(value) = x { if (value > 5) continue outer; t += value; }
  return t;
}
console.log(f([O.Some(1), O.Some(2), O.None, O.Some(9)]));
console.log(g(true, O.Some(1)), g(true, O.None), g(false, O.None));
console.log(h(true, O.Some(1)), h(true, O.None), h(false, O.None));
console.log(k([O.Some(1), O.Some(7), O.Some(2)]));

export {};


//// [runtimeAnIfLetAsAnUnbracedBodyKeepsItsParentAndItsElse.ts]

type O =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const O = {
  Some: (value: number): O => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
function f(xs: O[]): number {
  let t = 0;
  for (const x of xs) {
    const $tt_t0 = x;
    if ($tt_t0.kind === "Some") {
      const { value } = $tt_t0;
      t += value;
    } else {
      break;
    }
  }
  return t;
}
function g(c: boolean, x: O): number {
  if (c) {
    const $tt_t1 = x;
    if ($tt_t1.kind === "Some") {
      const { value } = $tt_t1;
      return value;
    } else {
      return 2;
    }
  }
  else { return 3; }
}
function h(c: boolean, x: O): number {
  if (c) {
    const $tt_t2 = x;
    if ($tt_t2.kind === "Some") {
      const { value } = $tt_t2;
      return value;
    } else {
      return 3;
    }
  }
  return 4;
}
function k(xs: O[]): number {
  let t = 0;
  outer: for (const x of xs) {
    const $tt_t3 = x;
    if ($tt_t3.kind === "Some") {
      const { value } = $tt_t3;
      if (value > 5) continue outer; t += value;
    }
  }
  return t;
}
console.log(f([O.Some(1), O.Some(2), O.None, O.Some(9)]));
console.log(g(true, O.Some(1)), g(true, O.None), g(false, O.None));
console.log(h(true, O.Some(1)), h(true, O.None), h(false, O.None));
console.log(k([O.Some(1), O.Some(7), O.Some(2)]));

export {};
