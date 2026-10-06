//// [inertArgumentsAreNotCapturedOutOfTheirContextualPosition3.tt] ////
consume({run: (n) => n}, match (x) { A(v) => v, _ => 0 });


//// [inertArgumentsAreNotCapturedOutOfTheirContextualPosition3.ts]
{
  const $tt_v1: typeof consume = (consume);
  const $tt_v2 = ({run: (n) => n});
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
