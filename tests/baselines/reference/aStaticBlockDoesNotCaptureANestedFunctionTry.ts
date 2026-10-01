//// [aStaticBlockDoesNotCaptureANestedFunctionTry.tt] ////
class C { static { const run = () => { try read(); }; } }


//// [aStaticBlockDoesNotCaptureANestedFunctionTry.ts]
class C { static { const run = () => { const $tt_t0 = read();
if (!("value" in $tt_t0)) {
  return $tt_t0;
} }; } }
