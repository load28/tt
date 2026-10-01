//// [ifLetDuplicateBindingIsAnError.tt] ////
function f() {
  if let Both(a: v, b: v) = o { g(v); }
}

