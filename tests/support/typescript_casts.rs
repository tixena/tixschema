//! Finds every `as` expression and type assertion in generated TypeScript, through the parser of
//! the TypeScript compiler itself, so a cast is told from the word "as" in a comment or a string.

/// The script that parses each `[name, text]` of `texts`, a JSON array, and prints one line per
/// cast it holds: the text's name, the line, and the cast as written.
pub fn driver(texts: &[(&str, String)]) -> String {
    let listed = serde_json::to_string(texts).unwrap();
    format!(
        r#"import ts from "typescript";
const texts = {listed};
const found = [];
for (const [name, text] of texts) {{
  const file = ts.createSourceFile(`${{name}}.ts`, text, ts.ScriptTarget.Latest, true);
  const walk = (node) => {{
    if (ts.isAsExpression(node) || ts.isTypeAssertionExpression(node)) {{
      const line = file.getLineAndCharacterOfPosition(node.getStart()).line + 1;
      found.push(`${{name}}:${{line}}: ${{node.getText().replace(/\s+/g, " ").slice(0, 160)}}`);
    }}
    ts.forEachChild(node, walk);
  }};
  walk(file);
}}
console.log(found.join("\n"));
"#
    )
}

/// The casts a run of [`driver`] printed, one per line.
pub fn found(printed: &str) -> Vec<&str> {
    printed.lines().filter(|line| !line.is_empty()).collect()
}
