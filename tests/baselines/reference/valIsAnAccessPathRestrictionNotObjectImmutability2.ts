//// [valIsAnAccessPathRestrictionNotObjectImmutability2.tt] ////
let original = { count: 0 };
val const view = original;
view.count++;

