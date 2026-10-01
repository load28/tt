//// [directReturnMatchDoesNotRequireASemicolon.tt] ////
function f(x: T) {
  return match (x) { A(value) => value, _ => 0 }
}


//// [directReturnMatchDoesNotRequireASemicolon.ts]
function f(x: T) {
  let $tt_v0;
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
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
  return $tt_v0
}
