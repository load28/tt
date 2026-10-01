//// [runtimeResultBlockWithAwaitResolvesToAResult.tt] ////

variant Res<T, E> { Ok(value: T), Err(error: E) }

const fetchNum = async (n: number): Promise<Res<number, string>> =>
  n > 0 ? Res.Ok(n) : Res.Err("not positive");

const total = async (a: number, b: number) => result {
  const x = try await fetchNum(a);
  const y = try await fetchNum(b);
  return x + y;
};

total(2, 3).then((r) => console.log(JSON.stringify(r)));
total(2, -1).then((r) => console.log(JSON.stringify(r)));

export {};


//// [runtimeResultBlockWithAwaitResolvesToAResult.ts]

type Res<T, E> =
  | { kind: "Ok"; value: T }
  | { kind: "Err"; error: E };
const Res = {
  Ok: <T, E>(value: T): Res<T, E> => ({ kind: "Ok", value }),
  Err: <T, E>(error: E): Res<T, E> => ({ kind: "Err", error }),
};

const fetchNum = async (n: number): Promise<Res<number, string>> =>
  n > 0 ? Res.Ok(n) : Res.Err("not positive");

const total = async (a: number, b: number) => {
  let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v0: {
    const $tt_t0 = await fetchNum(a);
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const x = $tt_t0.value;
  const $tt_t1 = await fetchNum(b);
  if (!("value" in $tt_t1)) {
    $tt_v0 = $tt_t1;
    break $tt_v0;
  }
  const y = $tt_t1.value;
  {
    const $tt_a0 = { value: { kind: "Ok" as const, value: x + y } };
    $tt_v0 = $tt_a0.value;
    break $tt_v0;
  }
  }
  return $tt_v0;
};

total(2, 3).then((r) => console.log(JSON.stringify(r)));
total(2, -1).then((r) => console.log(JSON.stringify(r)));

export {};
