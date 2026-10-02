//// [runtimeResultPreservesStatementMatchEffectOrder.tt] ////

variant Res<T, E> { Ok(value: T), Err(error: E) }
const events: string[] = [];
const read = (): Res<number, string> => Res.Ok(7);
const subject = (tag: number) => { events.push("subject-" + tag); return tag; };

const run = (tag: number) => result {
  const value = try read();
  match (subject(tag)) {
    1 => { events.push("one"); },
    _ => { events.push("other"); },
  }
  events.push("after");
  return value;
};

console.log(JSON.stringify(run(1)), events.join(","));
events.length = 0;
console.log(JSON.stringify(run(2)), events.join(","));

export {};


//// [runtimeResultPreservesStatementMatchEffectOrder.ts]

type Res<T, E> =
  | { kind: "Ok"; value: T }
  | { kind: "Err"; error: E };
const Res = {
  Ok: <T, E>(value: T): Res<T, E> => ({ kind: "Ok", value }),
  Err: <T, E>(error: E): Res<T, E> => ({ kind: "Err", error }),
};
const events: string[] = [];
const read = (): Res<number, string> => Res.Ok(7);
const subject = (tag: number) => { events.push("subject-" + tag); return tag; };

const run = (tag: number) => {
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
    const value = $tt_t0.value;
  let $tt_v1: undefined;
  {
    const $tt_m = subject(tag);
    switch ($tt_m) {
      case 1: {
        events.push("one");
          $tt_v1 = undefined;
          break;
      }
      default: {
        events.push("other");
          $tt_v1 = undefined;
          break;
      }
    }
  }
  
  events.push("after");
  {
    const $tt_a0 = { value: { kind: "Ok" as const, value: value } };
    $tt_v0 = $tt_a0.value;
    break $tt_v0;
  }
  }
  return $tt_v0;
};

console.log(JSON.stringify(run(1)), events.join(","));
events.length = 0;
console.log(JSON.stringify(run(2)), events.join(","));

export {};
