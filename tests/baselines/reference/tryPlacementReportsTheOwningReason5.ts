//// [tryPlacementReportsTheOwningReason5.tt] ////
const value = match (source) { Ok(value) => { const item = try read(); return item; }, Err(error) => error };

