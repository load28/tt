//// [tryInsideAFunctionInsideATemplateInterpolationIsAllowed.tt] ////
const s = `${run(() => { try g(); return h(); })}`;


//// [tryInsideAFunctionInsideATemplateInterpolationIsAllowed.ts]
const s = `${run(() => { const $tt_t0 = g();
if (!("value" in $tt_t0)) {
  return $tt_t0;
} return h(); })}`;
