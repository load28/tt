//! Install the current compiler as the project's real tt content mapper.

use std::path::Path;

pub fn install(root: &Path) {
    let package = root.join("node_modules/@openload28/tt-lang");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("package.json"),
        serde_json::json!({
            "name": "@openload28/tt-lang",
            "version": "0.0.0-test",
            "typescript": { "contentMapper": {
                "exec": [env!("CARGO_BIN_EXE_ttc"), "--content-mapper"]
            } }
        })
        .to_string(),
    )
    .unwrap();
    let config = root.join("tsconfig.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    value["contentMappers"] = serde_json::json!([
        { "package": "@openload28/tt-lang", "extensions": [".tt", ".ttx"] }
    ]);
    std::fs::write(config, value.to_string()).unwrap();
}
