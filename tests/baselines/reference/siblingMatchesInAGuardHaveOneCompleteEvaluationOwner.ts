//// [siblingMatchesInAGuardHaveOneCompleteEvaluationOwner.tt] ////
const x = match (1) { 1 if (match (2) { 2 => true, _ => false }) && (match (3) { 3 => true, _ => false }) => 10, _ => 20 };


//// [siblingMatchesInAGuardHaveOneCompleteEvaluationOwner.ts]
let $tt_v0$x: number;
{
  const $tt_m = 1;
  do {
    if ($tt_m === 1) {
      let $tt_subject_1;
      let $tt_subject_2;
      if ((($tt_subject_1 = 2, ($tt_subject_1 === 2) ? true : false)) && (($tt_subject_2 = 3, ($tt_subject_2 === 3) ? true : false))) {
        $tt_v0$x = 10;
        break;
      }
    }
    $tt_v0$x = 20;
    break;
  } while (false);
}
const x = $tt_v0$x;
