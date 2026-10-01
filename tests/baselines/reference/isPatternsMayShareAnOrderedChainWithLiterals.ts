//// [isPatternsMayShareAnOrderedChainWithLiterals.tt] ////
const value = match (x) { is Error => "error", "ok" => "ok", _ => "other" };


//// [isPatternsMayShareAnOrderedChainWithLiterals.ts]
let $tt_v0$value: string;
{
  const $tt_m = x;
  do {
    if ($tt_m instanceof Error) {
      $tt_v0$value = "error";
      break;
    }
    if ($tt_m === "ok") {
      $tt_v0$value = "ok";
      break;
    }
    $tt_v0$value = "other";
    break;
  } while (false);
}
const value = $tt_v0$value;
