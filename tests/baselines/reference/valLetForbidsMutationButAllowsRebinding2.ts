//// [valLetForbidsMutationButAllowsRebinding2.tt] ////
val let state = { count: 0 };
state = { ...state, count: state.count + 1 };


//// [valLetForbidsMutationButAllowsRebinding2.ts]
let state = { count: 0 };
state = { ...state, count: state.count + 1 };
