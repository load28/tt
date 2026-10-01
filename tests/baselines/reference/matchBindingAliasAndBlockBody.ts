//// [matchBindingAliasAndBlockBody.tt] ////

const r = match (m) {
  Move(x: px, y: py) => {
    const sum = px + py;
    return sum;
  },
  _ => 0,
};


//// [matchBindingAliasAndBlockBody.ts]

let $tt_v0$r;
{
  const $tt_m = m;
  switch ($tt_m.kind) {
    case "Move": {
      const { x: px, y: py } = $tt_m;
      const sum = px + py;
      $tt_v0$r = sum;
      break;
    }
    default: {
      $tt_v0$r = 0;
      break;
    }
  }
}
const r = $tt_v0$r;
