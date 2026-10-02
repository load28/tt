//// [tryBindsToAPrivateMemberOperand.tt] ////
variant R { Ok(value: number), Err(error: string) }
class C {
#v: R = R.Ok(1);
#c: C = this;
#case: R = R.Ok(1);
#match(): R { return R.Ok(2); }
f(): R {
const a = try this.#v;
try this.#v;
const b = try this.#c?.#v;
try this.#case;
const d = try this.#match() * 2;
return R.Ok(a + b + d);
}
}


//// [tryBindsToAPrivateMemberOperand.ts]
type R =
  | { kind: "Ok"; value: number }
  | { kind: "Err"; error: string };
const R = {
  Ok: (value: number): R => ({ kind: "Ok", value }),
  Err: (error: string): R => ({ kind: "Err", error }),
};
class C {
#v: R = R.Ok(1);
#c: C = this;
#case: R = R.Ok(1);
#match(): R { return R.Ok(2); }
f(): R {
const $tt_t0 = this.#v;
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
const a = $tt_t0.value;
const $tt_t1 = this.#v;
if (!("value" in $tt_t1)) {
  return $tt_t1;
}
const $tt_t2 = this.#c?.#v;
if (!("value" in $tt_t2)) {
  return $tt_t2;
}
const b = $tt_t2.value;
const $tt_t3 = this.#case;
if (!("value" in $tt_t3)) {
  return $tt_t3;
}
let $tt_v0: number;
const $tt_t4 = this.#match();
if (!("value" in $tt_t4)) {
  return $tt_t4;
}
$tt_v0 = $tt_t4.value;
const d = $tt_v0 * 2;
return R.Ok(a + b + d);
}
}
