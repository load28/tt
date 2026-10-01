//// [ifLetEmitsASelfContainedBlock.tt] ////
function f() {
  if let Some(value: user) = find() {
    greet(user);
  }
}


//// [ifLetEmitsASelfContainedBlock.ts]
function f() {
  {
    const $tt_t0 = find();
    if ($tt_t0.kind === "Some") {
      const { value: user } = $tt_t0;
      greet(user);
    }
  }
}
