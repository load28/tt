//// [runtimeIsPatternsAndLoopTestRegionsPreserveOrderAndCount.tt] ////

class Keep extends Error {}
class Stop extends Error {}
let probes = 0;
let updates = 0;
let bodies = 0;
function probe(): Error {
  probes += 1;
  return probes <= 3 ? new Keep() : new Stop();
}
for (; match (probe()) { is Keep => true, _ => false }; updates += 1) {
  bodies += 1;
  if (bodies < 3) continue;
}
const message = match (new SyntaxError("bad")) {
  is SyntaxError { message } if message.length > 0 => message,
  is Error { message: detail } => detail,
  _ => "unknown",
};
console.log(probes, updates, bodies, message);

export {};


//// [runtimeIsPatternsAndLoopTestRegionsPreserveOrderAndCount.ts]

class Keep extends Error {}
class Stop extends Error {}
let probes = 0;
let updates = 0;
let bodies = 0;
function probe(): Error {
  probes += 1;
  return probes <= 3 ? new Keep() : new Stop();
}
for (; ; updates += 1) {
  let $tt_v0: boolean;
  {
    const $tt_m = probe();
    do {
      if ($tt_m instanceof Keep) {
        $tt_v0 = true;
        break;
      }
      $tt_v0 = false;
      break;
    } while (false);
  }
  if (!($tt_v0)) break; {
  bodies += 1;
  if (bodies < 3) continue;
}}
let $tt_v1: string;
{
  const $tt_m = new SyntaxError("bad");
  do {
    if ($tt_m instanceof SyntaxError) {
      const { message } = $tt_m;
      if (message.length > 0) {
        $tt_v1 = message;
        break;
      }
    }
    if ($tt_m instanceof Error) {
      const { message: detail } = $tt_m;
      $tt_v1 = detail;
      break;
    }
    $tt_v1 = "unknown";
    break;
  } while (false);
}
const message = $tt_v1;
console.log(probes, updates, bodies, message);

export {};
