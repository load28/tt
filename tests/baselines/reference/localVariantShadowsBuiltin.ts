//// [localVariantShadowsBuiltin.tt] ////
variant Option { Some(), Stale }
const f = (o: Option) => match (o) { Some => 1 };

