//// [wildcardExemptsBuiltinExhaustiveness.tt] ////
const f = (o: Option<number>) => match (o) { Some(value) => value, _ => 0 };


//// [wildcardExemptsBuiltinExhaustiveness.ts]
const f = (o: Option<number>) => {
  let $tt_v0;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "Some": {
        const { value } = $tt_m;
        $tt_v0 = value;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  return $tt_v0;
};
