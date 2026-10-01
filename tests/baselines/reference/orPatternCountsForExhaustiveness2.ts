//// [orPatternCountsForExhaustiveness2.tt] ////

variant Dir { North(), South, East, West }
const f = (d: Dir) => match (d) {
  North | South => 1,
  East => 2,
};

