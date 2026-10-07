//// [resultPreservesAStatementPositionMatchDispatch.tt] ////
const value = result { const item = try read(); match (item) { 1 => useOne(), _ => useOther() }; return item; };


//// [resultPreservesAStatementPositionMatchDispatch.ts]
let $tt_v0$value;
$tt_v0$value: {
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    $tt_v0$value = $tt_t0;
    break $tt_v0$value;
  }
  const item = $tt_t0.value; let $tt_v1;
  {
    const $tt_m = item;
    switch ($tt_m) {
      case 1: {
        $tt_v1 = useOne();
        break;
      }
      default: {
        $tt_v1 = useOther();
        break;
      }
    }
  }
  ; { $tt_v0$value = { kind: "Ok" as const, value: item }; break $tt_v0$value; }
}
const value = $tt_v0$value;
