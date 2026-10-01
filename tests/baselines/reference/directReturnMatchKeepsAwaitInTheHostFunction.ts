//// [directReturnMatchKeepsAwaitInTheHostFunction.tt] ////
async function f(x: T) { return match (x) { A(url) => await fetch(url), _ => null }; }


//// [directReturnMatchKeepsAwaitInTheHostFunction.ts]
async function f(x: T) { let $tt_v0: (Response) | (null);
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "A": {
      const { url } = $tt_m;
      $tt_v0 = await fetch(url);
      break;
    }
    default: {
      $tt_v0 = null;
      break;
    }
  }
}
return $tt_v0; }
\ No newline at end of file
