//// [pipelineConciseArrowKeepsTryInTheArrow.tt] ////
const f = value |> (x => try next());


//// [pipelineConciseArrowKeepsTryInTheArrow.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
const f = $tt_ap(value, ((x => {
  let $tt_v0;
  const $tt_t0 = next();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  return $tt_v0;
})));
