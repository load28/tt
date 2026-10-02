//// [statementBodiedResultRequiresASuccessReturn.tt] ////
const value = result { const item = try read(); use(item); };

