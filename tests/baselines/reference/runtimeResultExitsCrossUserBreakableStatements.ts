//// [runtimeResultExitsCrossUserBreakableStatements.tt] ////

variant Res<T, E> { Ok(value: T), Err(error: E) }
const events: string[] = [];
const step = (ok: boolean, name: string): Res<number, string> =>
  ok ? Res.Ok(name.length) : Res.Err(name);

const fromFor = (ok: boolean) => result {
  for (const name of ["for"]) { return try step(ok, name); }
  events.push("for-tail");
  return 99;
};
const fromWhile = (ok: boolean) => result {
  while (true) { return try step(ok, "while"); }
  events.push("while-tail");
  return 99;
};
const fromDo = (ok: boolean) => result {
  do { return try step(ok, "do"); } while (false);
  events.push("do-tail");
  return 99;
};
const fromSwitch = (ok: boolean) => result {
  switch (ok) { default: return try step(ok, "switch"); }
  events.push("switch-tail");
  return 99;
};

for (const run of [fromFor, fromWhile, fromDo, fromSwitch]) {
  console.log(JSON.stringify(run(true)), JSON.stringify(run(false)));
}
console.log(events.join(","));

export {};


//// [runtimeResultExitsCrossUserBreakableStatements.ts]

type Res<T, E> =
  | { kind: "Ok"; value: T }
  | { kind: "Err"; error: E };
const Res = {
  Ok: <T, E>(value: T): Res<T, E> => ({ kind: "Ok", value }),
  Err: <T, E>(error: E): Res<T, E> => ({ kind: "Err", error }),
};
const events: string[] = [];
const step = (ok: boolean, name: string): Res<number, string> =>
  ok ? Res.Ok(name.length) : Res.Err(name);

const fromFor = (ok: boolean) => {
  let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v0: {
    for (const name of ["for"]) { const $tt_t0 = step(ok, name);
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const $tt_a0 = { value: { kind: "Ok" as const, value: $tt_t0.value } };
    $tt_v0 = $tt_a0.value;
    break $tt_v0; }
  events.push("for-tail");
  {
    const $tt_a1 = { value: { kind: "Ok" as const, value: 99 } };
    $tt_v0 = $tt_a1.value;
    break $tt_v0;
  }
  }
  return $tt_v0;
};
const fromWhile = (ok: boolean) => {
  let $tt_v1: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v1: {
    while (true) { const $tt_t1 = step(ok, "while");
    if (!("value" in $tt_t1)) {
      $tt_v1 = $tt_t1;
      break $tt_v1;
    }
    const $tt_a2 = { value: { kind: "Ok" as const, value: $tt_t1.value } };
    $tt_v1 = $tt_a2.value;
    break $tt_v1; }
  events.push("while-tail");
  {
    const $tt_a3 = { value: { kind: "Ok" as const, value: 99 } };
    $tt_v1 = $tt_a3.value;
    break $tt_v1;
  }
  }
  return $tt_v1;
};
const fromDo = (ok: boolean) => {
  let $tt_v2: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v2: {
    do { const $tt_t2 = step(ok, "do");
    if (!("value" in $tt_t2)) {
      $tt_v2 = $tt_t2;
      break $tt_v2;
    }
    const $tt_a4 = { value: { kind: "Ok" as const, value: $tt_t2.value } };
    $tt_v2 = $tt_a4.value;
    break $tt_v2; } while (false);
  events.push("do-tail");
  {
    const $tt_a5 = { value: { kind: "Ok" as const, value: 99 } };
    $tt_v2 = $tt_a5.value;
    break $tt_v2;
  }
  }
  return $tt_v2;
};
const fromSwitch = (ok: boolean) => {
  let $tt_v3: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v3: {
    switch (ok) { default: const $tt_t3 = step(ok, "switch");
    if (!("value" in $tt_t3)) {
      $tt_v3 = $tt_t3;
      break $tt_v3;
    }
    const $tt_a6 = { value: { kind: "Ok" as const, value: $tt_t3.value } };
    $tt_v3 = $tt_a6.value;
    break $tt_v3; }
  events.push("switch-tail");
  {
    const $tt_a7 = { value: { kind: "Ok" as const, value: 99 } };
    $tt_v3 = $tt_a7.value;
    break $tt_v3;
  }
  }
  return $tt_v3;
};

for (const run of [fromFor, fromWhile, fromDo, fromSwitch]) {
  console.log(JSON.stringify(run(true)), JSON.stringify(run(false)));
}
console.log(events.join(","));

export {};
