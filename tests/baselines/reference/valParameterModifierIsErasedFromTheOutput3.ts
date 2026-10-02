//// [valParameterModifierIsErasedFromTheOutput3.tt] ////
function pick(a: A, val b: B, val { c }: C) {}


//// [valParameterModifierIsErasedFromTheOutput3.ts]
function pick(a: A, b: B, { c }: C) {}
