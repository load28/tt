//// [fileWithoutFlowGetsNoCompositionHelper.tt] ////
const a = x |> f;


//// [fileWithoutFlowGetsNoCompositionHelper.ts]
const a = (($tt_v, $tt_f) => $tt_f($tt_v))(x, f);
