//// [valLeavesReadsAndComparisonsAlone.tt] ////
val const x = load();
const r = [x.a == 1, x.a === 1, x.a != 1, x.a >= 1, x.a <= 1, x.a && 1, x.a || 1, x.a ?? 1, x.a + 1, x.a > 1];
const y = x.a;
const z = { ...x };


//// [valLeavesReadsAndComparisonsAlone.ts]
const x = load();
const r = [x.a == 1, x.a === 1, x.a != 1, x.a >= 1, x.a <= 1, x.a && 1, x.a || 1, x.a ?? 1, x.a + 1, x.a > 1];
const y = x.a;
const z = { ...x };
