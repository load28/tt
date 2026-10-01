//// [aResultReturnWithoutASemicolonCompletesTheBlock.tt] ////
declare function read(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
export function f() {
  const r = result {
    const a = try read();
    return a
  };
  return r;
}


//// [aResultReturnWithoutASemicolonCompletesTheBlock.ts]
declare function read(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
export function f() {
  let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v0: {
    const $tt_t0 = read();
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const a = $tt_t0.value;
    {
      const $tt_a0 = { value: { kind: "Ok" as const, value: a } };
      $tt_v0 = $tt_a0.value;
      break $tt_v0;
    }
  }
  const r = $tt_v0;
  return r;
}
