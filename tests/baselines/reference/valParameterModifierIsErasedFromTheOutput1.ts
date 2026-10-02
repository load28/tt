//// [valParameterModifierIsErasedFromTheOutput1.tt] ////
function read(val user: User) {
  return user.name;
}


//// [valParameterModifierIsErasedFromTheOutput1.ts]
function read(user: User) {
  return user.name;
}
