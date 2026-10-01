//// [valConstForbidsMutationAtAnyDepth2.tt] ////
val const s = { u: { p: { n: 0 } } };
s.u.p.n += 1;

