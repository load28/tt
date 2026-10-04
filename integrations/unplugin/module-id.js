/** File-backed tt module IDs and bundler query markers. */
import * as fs from "node:fs";
import * as path from "node:path";

const TS_SUFFIX = ".ts";
const TSX_SUFFIX = ".tsx";

const TT_FILE = /\.ttx?$/;
const isFile = (file) => {
  try {
    return fs.statSync(file, { throwIfNoEntry: false })?.isFile() === true;
  } catch {
    return false;
  }
};

const fileOf = (id, base) => {
  for (let cut = id.length; cut > 0; cut = Math.max(id.lastIndexOf("?", cut - 1), id.lastIndexOf("#", cut - 1))) {
    const file = id.slice(0, cut);
    if (isFile(base === undefined ? file : path.resolve(base, file))) return file;
  }
  const name = Math.max(id.lastIndexOf("/"), id.lastIndexOf("\\")) + 1;
  const postfix = id.slice(name).search(/[?#]/);
  return postfix === -1 ? id : id.slice(0, name + postfix);
};
const SPECIAL_QUERY = /[?&](?:worker|sharedworker|raw|url)\b/;
const MODULE_MARKERS = new Set([`lang${TS_SUFFIX}`, `lang${TSX_SUFFIX}`]);
const moduleMarker = (file) => (file.endsWith(".ttx") ? `lang${TSX_SUFFIX}` : `lang${TS_SUFFIX}`);

const SCANNED_FILE = /\.ttx?(?:\?[^/]*)?$/;

/** A tt module id: the file, then a query ending in its marker. */
const TT_MODULE_ID = /(\.tt\?(?:[^#]*&)?lang\.ts|\.ttx\?(?:[^#]*&)?lang\.tsx)$/;

const queryOf = (id, file) => id.slice(file.length).replace(/#[\s\S]*$/, "");

const moduleId = (file, query) => {
  const params = query.slice(1).split("&").filter((param) => param !== "" && !MODULE_MARKERS.has(param));
  return `${file}?${[...params, moduleMarker(file)].join("&")}`;
};

const sourceFileOfId = (id) => {
  const file = fileOf(id);
  if (!TT_FILE.test(file)) return null;
  const params = queryOf(id, file).slice(1).split("&");
  return params[params.length - 1] === moduleMarker(file) ? file : null;
};

export { TS_SUFFIX, TSX_SUFFIX, TT_FILE, SPECIAL_QUERY, SCANNED_FILE, TT_MODULE_ID, fileOf, queryOf, moduleId, sourceFileOfId };
