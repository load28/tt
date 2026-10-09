//// [aValPassJudgesTheDeclarationTheCalleeResolvesTo.tt] ////
type Cfg = { a: number };
function h(y: Cfg) { y.a = 1; }
export function outer(val cfg: Cfg) { h(cfg); }
export function inner(val cfg: Cfg) {
  function h(val y: Cfg) { return y; }
  return h(cfg);
}
export function shadowed(val cfg: Cfg, h: (y: Cfg) => void) { h(cfg); }
export function later(val cfg: Cfg) { return set(cfg); }
const set = (y: Cfg) => { y.a = 2; };

