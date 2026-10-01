//// [noSubstitutionTemplateDynamicImportIsRewritten1.tt] ////
const m = import(`./x.tt`);


//// [noSubstitutionTemplateDynamicImportIsRewritten1.ts]
const m = import(`./x.js`);
