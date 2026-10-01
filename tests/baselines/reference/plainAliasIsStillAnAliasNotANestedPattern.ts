//// [plainAliasIsStillAnAliasNotANestedPattern.tt] ////
const n = match (o) { Some(value: None) => None, _ => 0 };


//// [plainAliasIsStillAnAliasNotANestedPattern.ts]
let $tt_v0$n;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "Some": {
      const { value: None } = $tt_m;
      $tt_v0$n = None;
      break;
    }
    default: {
      $tt_v0$n = 0;
      break;
    }
  }
}
const n = $tt_v0$n;
\ No newline at end of file
