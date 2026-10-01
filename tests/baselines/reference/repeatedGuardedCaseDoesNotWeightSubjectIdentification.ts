//// [repeatedGuardedCaseDoesNotWeightSubjectIdentification.tt] ////
variant Left { Alpha }
variant Right { Alphx }
const v = match (u) {
  Alpha if first => 1,
  Alpha => 2,
  Alphx => 3,
  _ => 4,
};


//// [repeatedGuardedCaseDoesNotWeightSubjectIdentification.ts]
type Left =
  { kind: "Alpha" };
const Left = {
  Alpha: { kind: "Alpha" } as const,
};
type Right =
  { kind: "Alphx" };
const Right = {
  Alphx: { kind: "Alphx" } as const,
};
let $tt_v0$v: number;
{
  const $tt_m = u;
  do {
    if ($tt_m.kind === "Alpha") {
      if (first) {
        $tt_v0$v = 1;
        break;
      }
    }
    if ($tt_m.kind === "Alpha") {
      $tt_v0$v = 2;
      break;
    }
    if ($tt_m.kind === "Alphx") {
      $tt_v0$v = 3;
      break;
    }
    $tt_v0$v = 4;
    break;
  } while (false);
}
const v = $tt_v0$v;
