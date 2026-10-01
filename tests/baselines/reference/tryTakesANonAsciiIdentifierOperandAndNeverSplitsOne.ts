//// [tryTakesANonAsciiIdentifierOperandAndNeverSplitsOne.tt] ////
declare function étry(): Result<number, string>;
function r(): Result<number, string> {
  const n = try étry();
  return Ok(n);
}


//// [tryTakesANonAsciiIdentifierOperandAndNeverSplitsOne.ts]
declare function étry(): Result<number, string>;
function r(): Result<number, string> {
  const $tt_t0 = étry();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const n = $tt_t0.value;
  return Ok(n);
}
