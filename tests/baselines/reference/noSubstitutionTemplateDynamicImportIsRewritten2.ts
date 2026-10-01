//// [noSubstitutionTemplateDynamicImportIsRewritten2.tt] ////
const m = import(`../view.ttx`, { with: { type: 'module' } });


//// [noSubstitutionTemplateDynamicImportIsRewritten2.ts]
const m = import(`../view.jsx`, { with: { type: 'module' } });
