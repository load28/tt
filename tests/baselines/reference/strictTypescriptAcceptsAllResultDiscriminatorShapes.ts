//// [strictTypescriptAcceptsAllResultDiscriminatorShapes.tt] ////

variant Res<T, E> { Ok(value: T), Err(error: E) }
type Alias<T, E> = Res<T, E>;

const directErr = () => {
  const value = try Res.Err("direct");
  return Res.Ok(value);
};
const directOk = () => {
  const value = try Res.Ok(1);
  return Res.Ok(value + 1);
};
const widened = (input: Res<number, string>): Res<number, string> => {
  const value = try input;
  return Res.Ok(value + 1);
};
const aliased = (input: Alias<number, string>): Alias<number, string> => {
  const value = try input;
  return Res.Ok(value + 1);
};
function generic<T, E>(input: Res<T, E>): Res<T, E> {
  const value = try input;
  return Res.Ok(value);
}

console.log(JSON.stringify(directErr()));
console.log(JSON.stringify(directOk()));
console.log(JSON.stringify(widened(Res.Err("wide"))));
console.log(JSON.stringify(aliased(Res.Ok(2))));
console.log(JSON.stringify(generic(Res.Ok("generic"))));

export {};


//// [strictTypescriptAcceptsAllResultDiscriminatorShapes.ts]

type Res<T, E> =
  | { kind: "Ok"; value: T }
  | { kind: "Err"; error: E };
const Res = {
  Ok: <T, E>(value: T): Res<T, E> => ({ kind: "Ok", value }),
  Err: <T, E>(error: E): Res<T, E> => ({ kind: "Err", error }),
};
type Alias<T, E> = Res<T, E>;

const directErr = () => {
  const $tt_t0 = Res.Err("direct");
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const value = $tt_t0.value;
  return Res.Ok(value);
};
const directOk = () => {
  const $tt_t1 = Res.Ok(1);
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  const value = $tt_t1.value;
  return Res.Ok(value + 1);
};
const widened = (input: Res<number, string>): Res<number, string> => {
  const $tt_t2 = input;
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  const value = $tt_t2.value;
  return Res.Ok(value + 1);
};
const aliased = (input: Alias<number, string>): Alias<number, string> => {
  const $tt_t3 = input;
  if (!("value" in $tt_t3)) {
    return $tt_t3;
  }
  const value = $tt_t3.value;
  return Res.Ok(value + 1);
};
function generic<T, E>(input: Res<T, E>): Res<T, E> {
  const $tt_t4 = input;
  if (!("value" in $tt_t4)) {
    return $tt_t4;
  }
  const value = $tt_t4.value;
  return Res.Ok(value);
}

console.log(JSON.stringify(directErr()));
console.log(JSON.stringify(directOk()));
console.log(JSON.stringify(widened(Res.Err("wide"))));
console.log(JSON.stringify(aliased(Res.Ok(2))));
console.log(JSON.stringify(generic(Res.Ok("generic"))));

export {};
