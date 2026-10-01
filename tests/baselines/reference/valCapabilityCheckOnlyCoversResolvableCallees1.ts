//// [valCapabilityCheckOnlyCoversResolvableCallees1.tt] ////
import { save } from "./io.js";
function f(val user: User) {
  save(user);
  user.save();
}


//// [valCapabilityCheckOnlyCoversResolvableCallees1.ts]
import { save } from "./io.js";
function f(user: User) {
  save(user);
  user.save();
}
