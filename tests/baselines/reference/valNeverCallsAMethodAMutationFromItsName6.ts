//// [valNeverCallsAMethodAMutationFromItsName6.tt] ////
val const items: number[] = [];
const n = items.map((v) => v).filter(Boolean).length;


//// [valNeverCallsAMethodAMutationFromItsName6.ts]
const items: number[] = [];
const n = items.map((v) => v).filter(Boolean).length;
