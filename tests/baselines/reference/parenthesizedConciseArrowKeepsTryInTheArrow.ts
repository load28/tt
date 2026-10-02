//// [parenthesizedConciseArrowKeepsTryInTheArrow.tt] ////
const f = () => (try next());


//// [parenthesizedConciseArrowKeepsTryInTheArrow.ts]
const f = () => {
  let $tt_v0;
  const $tt_t0 = next();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  return ($tt_v0);
};
