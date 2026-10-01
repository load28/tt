//// [valIsCheckedInsideNestedTtConstructs1.tt] ////
val const cfg = { a: 1 };
const msg = `${(() => { cfg.a = 2; return 1; })()}`;

