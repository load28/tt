//// [runtimeLetElseBindsFromAnObjectLiteralInitializer.tt] ////

function f(n: number) {
  const Some(value: v) = { kind: "Some" as const, value: n } else { return -1; };
  return v;
}
function g(on: boolean) {
  const Some(value) = on ? { kind: "Some" as const, value: "on" } : { kind: "None" as const } else { return "off"; };
  return value;
}
console.log(f(3), g(true), g(false));

export {};


//// [runtimeLetElseBindsFromAnObjectLiteralInitializer.ts]

function f(n: number) {
  const $tt_t0 = { kind: "Some" as const, value: n };
  if ($tt_t0.kind !== "Some") {
    return -1;
  }
  const { value: v } = $tt_t0;
  return v;
}
function g(on: boolean) {
  const $tt_t1 = on ? { kind: "Some" as const, value: "on" } : { kind: "None" as const };
  if ($tt_t1.kind !== "Some") {
    return "off";
  }
  const { value } = $tt_t1;
  return value;
}
console.log(f(3), g(true), g(false));

export {};
