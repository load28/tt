//// [aMatchInsideALiteralArgumentCompletesTheCallFromItsArms3.tt] ////
consume({outer: {item: match (x) { A(v) => v, _ => 0 }}});


//// [aMatchInsideALiteralArgumentCompletesTheCallFromItsArms3.ts]
{
  const $tt_v1: typeof consume = (consume);
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        $tt_v1({outer: {item: v}});
        break;
      }
      default: {
        $tt_v1({outer: {item: 0}});
        break;
      }
    }
  }
  
}
\ No newline at end of file
