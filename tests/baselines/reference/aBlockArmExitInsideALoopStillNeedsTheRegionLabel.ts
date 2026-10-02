//// [aBlockArmExitInsideALoopStillNeedsTheRegionLabel.tt] ////
const v = match (s) { "a" => { for (const x of xs) { return x; } return 0; }, _ => 0 };


//// [aBlockArmExitInsideALoopStillNeedsTheRegionLabel.ts]
let $tt_v0$v;
$tt_y_v0$v: {
  const $tt_m = s;
  switch ($tt_m) {
    case "a": {
      for (const x of xs) { $tt_v0$v = x; break $tt_y_v0$v; } $tt_v0$v = 0; break $tt_y_v0$v;
    }
    default: {
      $tt_v0$v = 0;
      break;
    }
  }
}
const v = $tt_v0$v;
\ No newline at end of file
