//// [tryDeclarationInForInitializerRunsBeforeTheLoop.tt] ////
function f() { for (let i = try next();;) {} }


//// [tryDeclarationInForInitializerRunsBeforeTheLoop.ts]
function f() { const $tt_t0 = next();
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
for (let i = $tt_t0.value;;) {} }
