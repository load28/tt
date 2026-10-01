//// [anIfLetThatStartsAStatementStaysAStatement5.tt] ////
variant O { Some(value: number), None }
declare const o: O;
declare function g(x: unknown): number;
const x = `${(() => { if let Some(value) = o { return value; } return 0; })()}`;


//// [anIfLetThatStartsAStatementStaysAStatement5.ts]
type O =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const O = {
  Some: (value: number): O => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
declare const o: O;
declare function g(x: unknown): number;
const x = `${(() => { {
  const $tt_t0 = o;
  if ($tt_t0.kind === "Some") {
    const { value } = $tt_t0;
    return value;
  }
} return 0; })()}`;
