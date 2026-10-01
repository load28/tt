//// [plainIfStatementsPassThrough.tt] ////
if (x) { a(); } else if (y) { b(); } else { c(); }
const z = cond ? 1 : 2;


//// [plainIfStatementsPassThrough.ts]
if (x) { a(); } else if (y) { b(); } else { c(); }
const z = cond ? 1 : 2;
