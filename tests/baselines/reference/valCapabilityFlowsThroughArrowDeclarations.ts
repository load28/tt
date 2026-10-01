//// [valCapabilityFlowsThroughArrowDeclarations.tt] ////
const update = (user: User) => { user.name = "x"; };
function process(val user: User) {
  update(user);
}

