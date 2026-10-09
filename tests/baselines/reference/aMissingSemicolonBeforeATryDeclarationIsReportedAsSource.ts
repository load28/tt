//// [aMissingSemicolonBeforeATryDeclarationIsReportedAsSource.tt] ////
declare function r(): { kind: "Ok"; value: number }; declare function g(): void;
export function f() {
  g()  const a = try r();
  return { kind: "Ok" as const, value: a };
}

