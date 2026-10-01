//// [valConstForbidsMutationAtAnyDepth1.tt] ////
val const x = { nested: { a: 1 } };
x.nested.a = 2;

