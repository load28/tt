//// [directReturnLiteralMatchKeepsAwaitInTheHostFunction.tt] ////
async function f() { return match (s) { "a" => await g(), _ => null }; }


//// [directReturnLiteralMatchKeepsAwaitInTheHostFunction.ts]
async function f() { let $tt_v0;
{
  const $tt_m = s;
  switch ($tt_m) {
    case "a": {
      $tt_v0 = await g();
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
