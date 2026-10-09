//// [runtimeOrdinaryResultSuccessPreservesExpressionHostProtocols.tt] ////

variant Res<T, E> { Ok(value: T), Err(error: E) }
const read = (value: number): Res<number, string> => Res.Ok(value);

class FieldBox {
  field = result { const value = try read(1); return value; };
}
class ConstructorBox {
  outcome;
  constructor() {
    this.outcome = result { const value = try read(2); return value; };
  }
}
function withDefault(value = result { const item = try read(3); return item; }) {
  return value;
}
function* values() {
  yield result { const item = try read(4); return item; };
  yield "after";
}
const text = `value=${result { const item = try read(5); return item; }}`;

console.log(JSON.stringify(new FieldBox().field));
console.log(JSON.stringify(new ConstructorBox().outcome));
console.log(JSON.stringify(withDefault()));
console.log(Array.from(values()).map((value) => JSON.stringify(value)).join(","));
console.log(text);

export {};


//// [runtimeOrdinaryResultSuccessPreservesExpressionHostProtocols.ts]
function $tt_expr<T>(run: () => T): T { return run(); }

type Res<T, E> =
  | { kind: "Ok"; value: T }
  | { kind: "Err"; error: E };
const Res = {
  Ok: <T, E>(value: T): Res<T, E> => ({ kind: "Ok", value }),
  Err: <T, E>(error: E): Res<T, E> => ({ kind: "Err", error }),
};
const read = (value: number): Res<number, string> => Res.Ok(value);

class FieldBox {
  field = $tt_expr(() => {
    const $tt_t0 = read(1);
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    const value = $tt_t0.value; { return { kind: "Ok" as const, value: value }; }
    });
}
class ConstructorBox {
  outcome;
  constructor() {
    let $tt_v1: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
    $tt_v1: {
      const $tt_t1 = read(2);
      if (!("value" in $tt_t1)) {
        $tt_v1 = $tt_t1;
        break $tt_v1;
      }
      const value = $tt_t1.value; { $tt_v1 = { kind: "Ok" as const, value: value }; break $tt_v1; }
    }
    this.outcome = $tt_v1;
  }
}
function withDefault(value = $tt_expr(() => {
  const $tt_t2 = read(3);
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  const item = $tt_t2.value; { return { kind: "Ok" as const, value: item }; }
  })) {
  return value;
}
function* values() {
  let $tt_v3: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v3: {
    const $tt_t3 = read(4);
    if (!("value" in $tt_t3)) {
      $tt_v3 = $tt_t3;
      break $tt_v3;
    }
    const item = $tt_t3.value; { $tt_v3 = { kind: "Ok" as const, value: item }; break $tt_v3; }
  }
  yield $tt_v3;
  yield "after";
}
let $tt_v4: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
$tt_v4: {
  const $tt_t4 = read(5);
  if (!("value" in $tt_t4)) {
    $tt_v4 = $tt_t4;
    break $tt_v4;
  }
  const item = $tt_t4.value; { $tt_v4 = { kind: "Ok" as const, value: item }; break $tt_v4; }
}
const text = `value=${$tt_v4}`;

console.log(JSON.stringify(new FieldBox().field));
console.log(JSON.stringify(new ConstructorBox().outcome));
console.log(JSON.stringify(withDefault()));
console.log(Array.from(values()).map((value) => JSON.stringify(value)).join(","));
console.log(text);

export {};
