//// [valIsAnAccessPathRestrictionNotObjectImmutability1.tt] ////
let original = { count: 0 };
val const view = original;
original.count++;


//// [valIsAnAccessPathRestrictionNotObjectImmutability1.ts]
let original = { count: 0 };
const view = original;
original.count++;
