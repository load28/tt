//// [inertArgumentsAreNotCapturedOutOfTheirContextualPosition2.tt] ////
const items = [make(), match (x) { A(v) => v, _ => 0 }];


//// [inertArgumentsAreNotCapturedOutOfTheirContextualPosition2.ts]
let $tt_v0$items;
const $tt_v1$items = (make());
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "A": {
      const { v } = $tt_m;
      $tt_v0$items = v;
      break;
    }
    default: {
      $tt_v0$items = 0;
      break;
    }
  }
}
const items = [$tt_v1$items, $tt_v0$items];
\ No newline at end of file
