//// [statementBodiedResultRequiresSuccessOnEveryReachablePath.tt] ////
const value = result { const item = try read(); if (item) return item; log(item); };

