//// [invalidTypeScriptInATryOperandIsReportedAsSource.tt] ////
declare function r(n: number): { kind: "Ok"; value: number };
export function f() { const v = try r(1 2); return { kind: "Ok" as const, value: v }; }

