//// [aTryExitingAResultFromAParameterInitializer.tt] ////
// A `try` inside a `result` exits to that result, and is written as a
// statement of its own host owner. An arrow's parameter initializer takes no
// statements, so the `try` has nowhere to be written: a placement error, as
// for a `try` that exits a function from the same position.
declare function read(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
export function g() {
  return result { const e = (x = try read()) => x; return 1; };
}

