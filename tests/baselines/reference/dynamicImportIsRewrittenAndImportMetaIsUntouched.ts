//// [dynamicImportIsRewrittenAndImportMetaIsUntouched.tt] ////
const m = import("./x.tt");
const u = import.meta.url;


//// [dynamicImportIsRewrittenAndImportMetaIsUntouched.ts]
const m = import("./x.js");
const u = import.meta.url;
