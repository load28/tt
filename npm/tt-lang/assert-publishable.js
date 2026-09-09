"use strict";

const fs = require("node:fs");
const path = require("node:path");

function assertPublishable(root = __dirname) {
  const marker = path.join(root, "tt-dev.local.json");
  if (!fs.existsSync(marker)) return;

  throw new Error(
    `refusing to publish a local development package: remove ${marker} first`,
  );
}

if (require.main === module) {
  try {
    assertPublishable();
  } catch (error) {
    console.error(`@openload28/tt-lang: ${error.message}`);
    process.exitCode = 1;
  }
}

module.exports = { assertPublishable };
