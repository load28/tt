//// [propagationUsesATypeCleanStructuralResultDiscriminator.tt] ////
function run() { const value = try Result.Err("boom"); return Result.Ok(value); }


//// [propagationUsesATypeCleanStructuralResultDiscriminator.ts]
function run() { const $tt_t0 = Result.Err("boom");
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
const value = $tt_t0.value; return Result.Ok(value); }
