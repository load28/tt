//// [runtimeResultBlockShortCircuitsOnTheFirstErr.tt] ////

variant Res<T, E> { Ok(value: T), Err(error: E) }

const steps: string[] = [];
const step = (name: string, ok: boolean): Res<string, string> => {
  steps.push(name);
  return ok ? Res.Ok(name) : Res.Err("failed:" + name);
};

const chain = (secondOk: boolean) => result {
  const a = try step("a", true);
  const b = try step("b", secondOk);
  const c = try step("c", true);
  return a + b + c;
};

console.log(JSON.stringify(chain(true)), steps.join(","));
steps.length = 0;
console.log(JSON.stringify(chain(false)), steps.join(","));

export {};


//// [runtimeResultBlockShortCircuitsOnTheFirstErr.ts]

type Res<T, E> =
  | { kind: "Ok"; value: T }
  | { kind: "Err"; error: E };
const Res = {
  Ok: <T, E>(value: T): Res<T, E> => ({ kind: "Ok", value }),
  Err: <T, E>(error: E): Res<T, E> => ({ kind: "Err", error }),
};

const steps: string[] = [];
const step = (name: string, ok: boolean): Res<string, string> => {
  steps.push(name);
  return ok ? Res.Ok(name) : Res.Err("failed:" + name);
};

const chain = (secondOk: boolean) => {
  let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: string;
});
  $tt_v0: {
    const $tt_t0 = step("a", true);
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const a = $tt_t0.value;
  const $tt_t1 = step("b", secondOk);
  if (!("value" in $tt_t1)) {
    $tt_v0 = $tt_t1;
    break $tt_v0;
  }
  const b = $tt_t1.value;
  const $tt_t2 = step("c", true);
  if (!("value" in $tt_t2)) {
    $tt_v0 = $tt_t2;
    break $tt_v0;
  }
  const c = $tt_t2.value;
  {
    const $tt_a0 = { value: { kind: "Ok" as const, value: a + b + c } };
    $tt_v0 = $tt_a0.value;
    break $tt_v0;
  }
  }
  return $tt_v0;
};

console.log(JSON.stringify(chain(true)), steps.join(","));
steps.length = 0;
console.log(JSON.stringify(chain(false)), steps.join(","));

export {};
