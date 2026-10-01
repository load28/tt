//// [tagsOfAHandWrittenUnionAreNotResolutionErrors.tt] ////
type Msg = { kind: "Ping" } | { kind: "Pong"; n: number };
const a = match (m) { Ping => 0, Pong(n) => n, _ => -1 };


//// [tagsOfAHandWrittenUnionAreNotResolutionErrors.ts]
type Msg = { kind: "Ping" } | { kind: "Pong"; n: number };
let $tt_v0$a;
{
  const $tt_m = m;
  switch ($tt_m.kind) {
    case "Ping": {
      $tt_v0$a = 0;
      break;
    }
    case "Pong": {
      const { n } = $tt_m;
      $tt_v0$a = n;
      break;
    }
    default: {
      $tt_v0$a = -1;
      break;
    }
  }
}
const a = $tt_v0$a;
