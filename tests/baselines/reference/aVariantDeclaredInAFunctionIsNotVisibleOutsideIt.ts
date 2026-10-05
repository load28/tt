//// [aVariantDeclaredInAFunctionIsNotVisibleOutsideIt.tt] ////
variant A { X, Y, Z }
function f() {
  variant Q { X, Y }
  const inner = (v: Q) => match (v) { X => 1 };
  return inner;
}
export const g = (v: A) => match (v) { X => 1, Y => 2 };
export { f };

