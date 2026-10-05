//// [aMatchArmBindingShadowsAnOuterVal.tt] ////
variant O { Some(value: { a: number }), None }
val const x = { a: 1 };
export function f(o: O) {
  return match (o) { Some(value: x) => { x.a = 1; return 1; }, None => 0 };
}
export function g(o: O) {
  return match (o) { Some(value: y) => y.a, None => x.a = 3 };
}

