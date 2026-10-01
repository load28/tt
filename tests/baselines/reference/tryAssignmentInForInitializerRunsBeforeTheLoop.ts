//// [tryAssignmentInForInitializerRunsBeforeTheLoop.tt] ////
function f() { for (i = try next();;) {} }


//// [tryAssignmentInForInitializerRunsBeforeTheLoop.ts]
function f() { let $tt_v0;
const $tt_t0 = next();
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
$tt_v0 = $tt_t0.value;
for (i = $tt_v0;;) {} }
