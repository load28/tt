//// [aConditionalMatchUsesOneExitTarget.tt] ////
const value = match (item) {
  A if ready => { consume(); },
  _ => 0,
};


//// [aConditionalMatchUsesOneExitTarget.ts]
let $tt_v0$value: (undefined) | (number);
{
  const $tt_m = item;
  $tt_b: {
    if ($tt_m.kind === "A") {
      if (ready) {
        { consume();
          $tt_v0$value = undefined;
          break $tt_b;
        }
      }
    }
    $tt_v0$value = 0;
    break $tt_b;
  }
}
const value = $tt_v0$value;
