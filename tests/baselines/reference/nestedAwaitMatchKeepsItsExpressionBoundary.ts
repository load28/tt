//// [nestedAwaitMatchKeepsItsExpressionBoundary.tt] ////
async function f(x: T) { return consume(match (x) { A(url) => await fetch(url), _ => null }); }


//// [nestedAwaitMatchKeepsItsExpressionBoundary.ts]
async function f(x: T) { let $tt_v0;
const $tt_v1: typeof consume = (consume);
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "A": {
      const { url } = $tt_m;
      $tt_v0 = $tt_v1(await fetch(url));
      break;
    }
    default: {
      $tt_v0 = $tt_v1(null);
      break;
    }
  }
}
return $tt_v0; }
\ No newline at end of file
