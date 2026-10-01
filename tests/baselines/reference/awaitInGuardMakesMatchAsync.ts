//// [awaitInGuardMakesMatchAsync.tt] ////
async function f(x: T) { return match (x) { A(u) if await allowed(u) => 1, _ => 0 }; }


//// [awaitInGuardMakesMatchAsync.ts]
async function f(x: T) { let $tt_v0: number;
{
  const $tt_m = x;
  do {
    if ($tt_m.kind === "A") {
      const { u } = $tt_m;
      if (await allowed(u)) {
        $tt_v0 = 1;
        break;
      }
    }
    $tt_v0 = 0;
    break;
  } while (false);
}
return $tt_v0; }
\ No newline at end of file
