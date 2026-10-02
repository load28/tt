//// [guardedBlockBodyUsesLabeledBreak.tt] ////
const r = match (x) { A(v) if v > 0 => { log(v); }, _ => 0 };


//// [guardedBlockBodyUsesLabeledBreak.ts]
let $tt_v0$r: (undefined) | (number);
{
  const $tt_m = x;
  $tt_b: {
    if ($tt_m.kind === "A") {
      const { v } = $tt_m;
      if (v > 0) {
        { log(v);
          $tt_v0$r = undefined;
          break $tt_b;
        }
      }
    }
    $tt_v0$r = 0;
    break $tt_b;
  }
}
const r = $tt_v0$r;
\ No newline at end of file
