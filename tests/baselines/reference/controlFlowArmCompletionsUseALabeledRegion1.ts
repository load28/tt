//// [controlFlowArmCompletionsUseALabeledRegion1.tt] ////
consume(match (x) { A(v) => { for (const s of [1]) { if (s === v) return s; } return 0; }, _ => 0 });


//// [controlFlowArmCompletionsUseALabeledRegion1.ts]
{
  const $tt_v1 = (consume);
  $tt_y_v1: {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        for (const s of [1]) { if (s === v) { $tt_v1(s); break $tt_y_v1; } } $tt_v1(0); break $tt_y_v1;
      }
      default: {
        $tt_v1(0);
        break;
      }
    }
  }
  
}
\ No newline at end of file
