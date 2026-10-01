//// [aMisspelledCaseInATupleMatchPositionIsReported.tt] ////
variant Dir { North(dx: number), South }
variant Speed { Fast(v: number), Slow }
const n = match (d, s) {
  (North(dx), Fast(v)) => dx + v,
  (Nrth(dx), Slow) => dx,
  (South, _) => 3,
};

