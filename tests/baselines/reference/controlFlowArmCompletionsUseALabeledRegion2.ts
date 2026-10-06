//// [controlFlowArmCompletionsUseALabeledRegion2.tt] ////
consume(match (x) { A(v) => { try { return v; } finally { effect(); } }, _ => 0 });


//// [controlFlowArmCompletionsUseALabeledRegion2.ts]
{
  let $tt_v0;
  const $tt_v1: typeof consume = (consume);
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        try { $tt_v0 = v; break; } finally { effect(); }
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  $tt_v1($tt_v0);
}
\ No newline at end of file
