//// [generatorStatementOwnerAcceptsMatchInitializers.tt] ////
function* f(code: number): Generator<string> {
const line = match (code) { 200 => "ok", _ => "err" };
yield line;
}


//// [generatorStatementOwnerAcceptsMatchInitializers.ts]
function* f(code: number): Generator<string> {
let $tt_v0: string;
{
  const $tt_m = code;
  switch ($tt_m) {
    case 200: {
      $tt_v0 = "ok";
      break;
    }
    default: {
      $tt_v0 = "err";
      break;
    }
  }
}
const line = $tt_v0;
yield line;
}
