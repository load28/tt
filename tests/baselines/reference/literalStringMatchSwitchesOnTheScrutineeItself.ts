//// [literalStringMatchSwitchesOnTheScrutineeItself.tt] ////

const label = match (dir) {
  "north" => "N",
  "south" => "S",
  _ => "?",
};


//// [literalStringMatchSwitchesOnTheScrutineeItself.ts]

let $tt_v0$label: string;
{
  const $tt_m = dir;
  switch ($tt_m) {
    case "north": {
      $tt_v0$label = "N";
      break;
    }
    case "south": {
      $tt_v0$label = "S";
      break;
    }
    default: {
      $tt_v0$label = "?";
      break;
    }
  }
}
const label = $tt_v0$label;
