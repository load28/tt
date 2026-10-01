//// [anIfLetThatStartsAStatementStaysAStatement3.tt] ////
variant O { Some(value: number), None }
declare const o: O;
declare function g(x: unknown): number;
if (o) if let Some(value) = o { g(value); }


//// [anIfLetThatStartsAStatementStaysAStatement3.ts]
type O =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const O = {
  Some: (value: number): O => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
declare const o: O;
declare function g(x: unknown): number;
if (o) {
  const $tt_t0 = o;
  if ($tt_t0.kind === "Some") {
    const { value } = $tt_t0;
    g(value);
  }
}
