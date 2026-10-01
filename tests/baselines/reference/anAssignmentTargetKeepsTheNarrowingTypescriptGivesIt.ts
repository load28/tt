//// [anAssignmentTargetKeepsTheNarrowingTypescriptGivesIt.tt] ////

type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
declare function read(): R<number>;
export function assigned(): R<number> {
  const state: { value?: number } = {};
  state.value = try read();
  return { kind: "Ok", value: state.value.toFixed().length };
}
export function compound(): R<number> {
  const state: { value: number | string } = { value: 1 };
  state.value = 0;
  state.value += try read();
  return { kind: "Ok", value: state.value };
}
export class Holder {
  value;
  constructor(r: R<number>) {
    this.value = result { return try r; };
  }
}

export {};


//// [anAssignmentTargetKeepsTheNarrowingTypescriptGivesIt.ts]

type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
declare function read(): R<number>;
export function assigned(): R<number> {
  const state: { value?: number } = {};
  let $tt_v0: number | undefined;
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  state.value = $tt_v0;
  return { kind: "Ok", value: state.value.toFixed().length };
}
export function compound(): R<number> {
  const state: { value: number | string } = { value: 1 };
  state.value = 0;
  let $tt_v1: number;
  let $tt_v2 = (state.value);
  const $tt_t1 = read();
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v1 = $tt_t1.value;
  state.value = $tt_v2 += $tt_v1;
  return { kind: "Ok", value: state.value };
}
export class Holder {
  value;
  constructor(r: R<number>) {
    let $tt_v3: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
    $tt_v3: {
      const $tt_t2 = r;
      if (!("value" in $tt_t2)) {
        $tt_v3 = $tt_t2;
        break $tt_v3;
      }
      const $tt_a0 = { value: { kind: "Ok" as const, value: $tt_t2.value } };
      $tt_v3 = $tt_a0.value;
      break $tt_v3;
    }
    this.value = $tt_v3;
  }
}

export {};
