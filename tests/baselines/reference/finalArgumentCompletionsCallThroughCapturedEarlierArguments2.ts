//// [finalArgumentCompletionsCallThroughCapturedEarlierArguments2.tt] ////
pair(match (x) { A(v) => v, _ => 0 }, last());


//// [finalArgumentCompletionsCallThroughCapturedEarlierArguments2.ts]
{
  let $tt_v0;
  const $tt_v1 = (pair);
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        $tt_v0 = v;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  $tt_v1($tt_v0, last());
}
\ No newline at end of file
