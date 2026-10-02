//// [aMatchInsideALiteralArgumentCompletesTheCallFromItsArms4.tt] ////
const kept = consume({item: match (x) { A(v) => v, _ => 0 }});


//// [aMatchInsideALiteralArgumentCompletesTheCallFromItsArms4.ts]
let $tt_v0$kept;
const $tt_v1$kept = (consume);
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "A": {
      const { v } = $tt_m;
      $tt_v0$kept = $tt_v1$kept({item: v});
      break;
    }
    default: {
      $tt_v0$kept = $tt_v1$kept({item: 0});
      break;
    }
  }
}
const kept = $tt_v0$kept;
\ No newline at end of file
