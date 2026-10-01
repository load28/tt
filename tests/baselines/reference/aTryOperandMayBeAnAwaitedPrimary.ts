//// [aTryOperandMayBeAnAwaitedPrimary.tt] ////
declare function a(): Promise<{ kind: "Ok"; value: number } | { kind: "Err"; error: string }>;
async function g() { const x = try await a(); const y = try await a() * 2; return x + y; }


//// [aTryOperandMayBeAnAwaitedPrimary.ts]
declare function a(): Promise<{ kind: "Ok"; value: number } | { kind: "Err"; error: string }>;
async function g() { const $tt_t0 = await a();
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
const x = $tt_t0.value; let $tt_v0: number;
const $tt_t1 = await a();
if (!("value" in $tt_t1)) {
  return $tt_t1;
}
$tt_v0 = $tt_t1.value;
const y = $tt_v0 * 2; return x + y; }
