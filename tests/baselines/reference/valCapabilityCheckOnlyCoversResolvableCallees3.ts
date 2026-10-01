//// [valCapabilityCheckOnlyCoversResolvableCallees3.tt] ////
function update(user: User) { user.name = "x"; }
function f(val user: User) {
  update({ ...user });
}


//// [valCapabilityCheckOnlyCoversResolvableCallees3.ts]
function update(user: User) { user.name = "x"; }
function f(user: User) {
  update({ ...user });
}
