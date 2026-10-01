//// [oneShotForHeadersHoistMatchControlFlowOnce1.tt] ////
for (const value of match (source) { is Array => source, _ => [] }) { use(value); }


//// [oneShotForHeadersHoistMatchControlFlowOnce1.ts]
{
  const $tt_m = source;
  
  for (const value of (($tt_m instanceof Array) ? source : [])) { use(value); }
}
