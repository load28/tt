//// [finalArgumentCompletionsCallThroughCapturedEarlierArguments1.tt] ////
pair(first(), match (x) { A(v) => v, _ => 0 });


//// [finalArgumentCompletionsCallThroughCapturedEarlierArguments1.ts]
{
  const $tt_v1: typeof pair = (pair);
  const $tt_v2 = (first());
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        $tt_v1($tt_v2, v);
        break;
      }
      default: {
        $tt_v1($tt_v2, 0);
        break;
      }
    }
  }
  
}
\ No newline at end of file
