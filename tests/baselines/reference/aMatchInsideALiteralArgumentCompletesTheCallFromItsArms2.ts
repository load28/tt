//// [aMatchInsideALiteralArgumentCompletesTheCallFromItsArms2.tt] ////
consume([match (x) { A(v) => v, _ => 0 }]);


//// [aMatchInsideALiteralArgumentCompletesTheCallFromItsArms2.ts]
{
  const $tt_v1 = (consume);
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        $tt_v1([v]);
        break;
      }
      default: {
        $tt_v1([0]);
        break;
      }
    }
  }
  
}
\ No newline at end of file
