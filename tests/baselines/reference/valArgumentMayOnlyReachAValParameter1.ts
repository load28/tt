//// [valArgumentMayOnlyReachAValParameter1.tt] ////
function read(val user: User) { log(user.name); }
function update(user: User) { user.name = "Lee"; }
function process(val user: User) {
  read(user);
}


//// [valArgumentMayOnlyReachAValParameter1.ts]
function read(user: User) { log(user.name); }
function update(user: User) { user.name = "Lee"; }
function process(user: User) {
  read(user);
}
