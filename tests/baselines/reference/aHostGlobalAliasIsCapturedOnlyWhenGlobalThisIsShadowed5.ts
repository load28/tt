//// [aHostGlobalAliasIsCapturedOnlyWhenGlobalThisIsShadowed5.tt] ////
variant O { Some(value: number), None }
export function f(o: O, globalThis: unknown) { const Error = 5; return o.kind; }


//// [aHostGlobalAliasIsCapturedOnlyWhenGlobalThisIsShadowed5.ts]
type O =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const O = {
  Some: (value: number): O => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
export function f(o: O, globalThis: unknown) { const Error = 5; return o.kind; }
