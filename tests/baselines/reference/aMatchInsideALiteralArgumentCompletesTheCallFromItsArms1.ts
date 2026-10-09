//// [aMatchInsideALiteralArgumentCompletesTheCallFromItsArms1.tt] ////
consume({item: match (x) { A(v) => v, _ => 0 }});


//// [aMatchInsideALiteralArgumentCompletesTheCallFromItsArms1.ts]
{
  const $tt_v1: typeof consume = (consume);
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        $tt_v1({item: v});
        break;
      }
      default: {
        $tt_v1({item: 0});
        break;
      }
    }
  }
  
}
\ No newline at end of file
