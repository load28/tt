//// [ifLetTakesNestedPatterns.tt] ////
function f(r: Res) {
  if let Ok(value: Some(value: v)) = r { use(v); }
}


//// [ifLetTakesNestedPatterns.ts]
function f(r: Res) {
  {
    const $tt_t0 = r;
    if ($tt_t0.kind === "Ok" && $tt_t0.value.kind === "Some") {
      const { value: v } = $tt_t0.value;
      use(v);
    }
  }
}
