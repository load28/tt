//// [ifLetAllowedInStatementContexts.tt] ////

function f(x: X, o: O) {
  const r = match (x) {
    A => {
      if let Some(value) = o { return value; }
      return 0;
    },
    _ => 1,
  };
  return r;
}


//// [ifLetAllowedInStatementContexts.ts]

function f(x: X, o: O) {
  let $tt_v0;
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
        {
          const $tt_t0 = o;
          if ($tt_t0.kind === "Some") {
            const { value } = $tt_t0;
            $tt_v0 = value; break;
          }
        }
        $tt_v0 = 0;
        break;
      }
      default: {
        $tt_v0 = 1;
        break;
      }
    }
  }
  const r = $tt_v0;
  return r;
}
