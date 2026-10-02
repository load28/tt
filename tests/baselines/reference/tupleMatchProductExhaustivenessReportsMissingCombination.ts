//// [tupleMatchProductExhaustivenessReportsMissingCombination.tt] ////

variant Dir { North(), South }
variant Speed { Fast(), Slow }
const step = match (d, s) {
  (North, Fast) => 2,
  (North, Slow) => 1,
  (South, Fast) => -1,
};

