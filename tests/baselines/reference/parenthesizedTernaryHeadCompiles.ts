//// [parenthesizedTernaryHeadCompiles.tt] ////
const a = (c ? x : y) |> f;


//// [parenthesizedTernaryHeadCompiles.ts]
const a = (($tt_v, $tt_f) => $tt_f($tt_v))((c ? x : y), f);
