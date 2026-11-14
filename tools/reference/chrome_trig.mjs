// Makes Math.sin and Math.cos return Chromium 147's results, which Node 24
// does not reproduce (docs/framework-gaps.md): every call is looked up in
// crates/js/tests/fixtures/chrome_trig.json, recorded in headless Chrome by
// tools/chrome/trig_probe.mjs. An argument missing from the table is
// collected (and gives NaN); the generator then lists it in
// tools/chrome/trig_args.json and stops, and the probe records it on its
// next run:
//
//   node shape_fixtures.mjs                                # lists missing arguments
//   node ../chrome/trig_probe.mjs <chrome executable>       # records them
//   python -I ../chrome/trig_gaps.py                       # updates the gaps
//   node shape_fixtures.mjs                                # writes the fixtures
//
// Math.tan and Math.asin are left alone: Chrome 147 and Node 24 both use
// V8's fdlibm port for them and agreed on every argument measured.

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const enc = (v) => (Object.is(v, -0) ? "-0" : String(v));

export function useChromeTrig(root) {
  const { cases } = JSON.parse(readFileSync(join(root, "crates/js/tests/fixtures/chrome_trig.json"), "utf8"));
  const sin = new Map();
  const cos = new Map();
  for (const [x, s, c] of cases) {
    sin.set(x, Number(s));
    cos.set(x, Number(c));
  }
  const missing = new Set();
  const lookup = (table) => (value) => {
    const x = +value;
    if (!Number.isFinite(x)) return NaN;
    const key = enc(x);
    if (table.has(key)) return table.get(key);
    missing.add(key);
    return NaN;
  };
  Math.sin = lookup(sin);
  Math.cos = lookup(cos);

  // Adds the missing arguments to tools/chrome/trig_args.json; true if any.
  return function recordMissing() {
    if (missing.size === 0) return false;
    const file = join(root, "tools/chrome/trig_args.json");
    const known = existsSync(file) ? JSON.parse(readFileSync(file, "utf8")) : [];
    const all = [...new Set([...known, ...missing])];
    writeFileSync(file, `[\n${all.map((x) => `  ${JSON.stringify(x)}`).join(",\n")}\n]\n`);
    console.error(
      `${missing.size} Math.sin/Math.cos arguments have no Chrome result; listed in tools/chrome/trig_args.json. ` +
        "Run tools/chrome/trig_probe.mjs and tools/chrome/trig_gaps.py, then this generator again.",
    );
    return true;
  };
}
