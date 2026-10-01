//// [aWholePrimaryInitializerIsStillTheStatementForm.tt] ////
declare function a(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function g() { const x = try a(); const y = try (a()); return x + y; }


//// [aWholePrimaryInitializerIsStillTheStatementForm.ts]
declare function a(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function g() { const $tt_t0 = a();
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
const x = $tt_t0.value; const $tt_t1 = (a());
if (!("value" in $tt_t1)) {
  return $tt_t1;
}
const y = $tt_t1.value; return x + y; }
