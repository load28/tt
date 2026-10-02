//// [valArgumentMayOnlyReachAValParameter2.tt] ////
function read(val user: User) { log(user.name); }
function update(user: User) { user.name = "Lee"; }
function process(val user: User) {
  update(user);
}

