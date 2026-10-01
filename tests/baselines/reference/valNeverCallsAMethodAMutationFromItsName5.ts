//// [valNeverCallsAMethodAMutationFromItsName5.tt] ////
val const s = { u: { p: { tags: [] as string[] } } };
s.u.p.tags.push("tt");


//// [valNeverCallsAMethodAMutationFromItsName5.ts]
const s = { u: { p: { tags: [] as string[] } } };
s.u.p.tags.push("tt");
