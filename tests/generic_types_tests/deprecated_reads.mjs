// What the TypeScript language service, the service an editor asks, says of the modules named on
// the command line: every error in any of them, then each deprecated name the last one reads, by
// its code, in the order read.
import ts from "typescript";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const files = process.argv.slice(2).map((file) => resolve(file));
const options = {
  strict: true,
  module: ts.ModuleKind.NodeNext,
  moduleResolution: ts.ModuleResolutionKind.NodeNext,
  target: ts.ScriptTarget.ES2022,
  skipLibCheck: true,
  noEmit: true,
};
const service = ts.createLanguageService({
  getScriptFileNames: () => files,
  getScriptVersion: () => "1",
  getScriptSnapshot: (file) =>
    ts.sys.fileExists(file) ? ts.ScriptSnapshot.fromString(readFileSync(file, "utf8")) : undefined,
  getCurrentDirectory: () => process.cwd(),
  getCompilationSettings: () => options,
  getDefaultLibFileName: (given) => ts.getDefaultLibFilePath(given),
  fileExists: ts.sys.fileExists,
  readFile: ts.sys.readFile,
  readDirectory: ts.sys.readDirectory,
  directoryExists: ts.sys.directoryExists,
  getDirectories: ts.sys.getDirectories,
});
for (const file of files) {
  for (const found of service.getSemanticDiagnostics(file)) {
    console.log(`error ${found.code}: ${ts.flattenDiagnosticMessageText(found.messageText, " ")}`);
  }
}
const reader = files[files.length - 1];
const text = readFileSync(reader, "utf8");
for (const found of service.getSuggestionDiagnostics(reader)) {
  if (found.reportsDeprecated) {
    console.log(`${found.code} ${text.slice(found.start, found.start + found.length)}`);
  }
}
