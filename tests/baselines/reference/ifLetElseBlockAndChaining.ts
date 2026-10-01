//// [ifLetElseBlockAndChaining.tt] ////

function f() {
  if let Some(value) = a() {
    use1(value);
  } else if let Ok(value: v) = b() {
    use2(v);
  } else {
    fallback();
  }
}


//// [ifLetElseBlockAndChaining.ts]

function f() {
  {
    const $tt_t0 = a();
    if ($tt_t0.kind === "Some") {
      const { value } = $tt_t0;
      use1(value);
    } else {
      const $tt_t1 = b();
      if ($tt_t1.kind === "Ok") {
        const { value: v } = $tt_t1;
        use2(v);
      } else {
        fallback();
      }
    }
  }
}
