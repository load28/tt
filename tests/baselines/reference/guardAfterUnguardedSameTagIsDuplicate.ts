//// [guardAfterUnguardedSameTagIsDuplicate.tt] ////
const r = match (x) { A => 1, A if c => 2, _ => 0 };

