//// [aMutableArgumentMayReachAnyParameter.tt] ////
function read(val user: User) { log(user.name); }
function update(user: User) { user.name = "Lee"; }
function process(user: User) {
  read(user);
  update(user);
}


//// [aMutableArgumentMayReachAnyParameter.ts]
function read(user: User) { log(user.name); }
function update(user: User) { user.name = "Lee"; }
function process(user: User) {
  read(user);
  update(user);
}
