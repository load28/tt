//// [valCapabilityCheckOnlyCoversResolvableCallees2.tt] ////
function apply(user: User) {}
function apply(val user: User) {}
function f(val user: User) {
  apply(user);
}


//// [valCapabilityCheckOnlyCoversResolvableCallees2.ts]
function apply(user: User) {}
function apply(user: User) {}
function f(user: User) {
  apply(user);
}
