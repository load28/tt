//// [aValMutationCoversTheBinding.tt] ////
function f() {
  val const cfg = { a: 1 };
  cfg.a = 2;
}

