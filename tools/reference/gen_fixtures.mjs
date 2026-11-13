// Generates the reference fixtures the Rust port is tested against, by running
// the pinned npm packages (package-lock.json) under Node.
//
//   npm ci --ignore-scripts
//   node gen_fixtures.mjs          # write the fixtures
//   node gen_fixtures.mjs --check  # fail if the committed fixtures differ
//
// Numbers are written as JavaScript's String(n) (shortest round-trip digits),
// with "-0" for negative zero, so the Rust side can compare exact bits.

import { createRequire } from "node:module";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const here = dirname(fileURLToPath(import.meta.url));
const fixtures = join(here, "..", "..", "crates", "recharts", "tests", "fixtures");

const recharts = require("recharts/package.json");
const decimal = require("decimal.js-light/package.json");

// decimal.js-light keeps module-level state (precision, an `external` flag)
// that an exception thrown inside it can leave changed: Recharts recurses until
// the stack overflows for some inputs, and the overflow can land inside
// Decimal#log. Reload both packages after any error so cases stay independent.
let ticks;
let getDigitCount;
function load() {
  for (const key of Object.keys(require.cache)) {
    if (/node_modules[\\/](recharts|decimal\.js-light)[\\/]/.test(key)) delete require.cache[key];
  }
  ticks = require("recharts/lib/util/scale/getNiceTickValues.js");
  ({ getDigitCount } = require("recharts/lib/util/scale/util/arithmetic.js"));
}
load();

const num = (n) => (Object.is(n, -0) ? "-0" : String(n));

// Long results (rangeStep stops at 100,000 steps when the step is 0) are
// stored run-length encoded as [value, count] pairs.
function encode(values) {
  const strings = values.map(num);
  if (strings.length <= 1000) return { ticks: strings };
  const runs = [];
  for (const s of strings) {
    if (runs.length && runs[runs.length - 1][0] === s) runs[runs.length - 1][1]++;
    else runs.push([s, 1]);
  }
  return { runs };
}

function run(f) {
  try {
    return encode(f());
  } catch (error) {
    load();
    return { error: String(error.message) };
  }
}

// Deterministic pseudo-random numbers (32-bit LCG), so the case list is stable.
let seed = 0x5eed;
function rand() {
  seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
  return seed / 2 ** 32;
}
function pick(list) {
  return list[Math.floor(rand() * list.length)];
}

// A value a chart's data could produce: a few significant digits at some
// magnitude, sometimes a binary-unfriendly sum, sometimes negative.
function value() {
  const exponent = pick([-7, -4, -3, -2, -1, 0, 0, 1, 1, 2, 2, 2, 3, 3, 4, 5, 6, 9, 12]);
  const digits = pick([1, 1, 2, 2, 3, 4, 6, 17]);
  let v = Math.round(rand() * 10 ** digits) / 10 ** digits * 10 ** exponent;
  if (rand() < 0.1) v = v + 0.1 + 0.2;
  if (rand() < 0.3) v = -v;
  return v;
}

// Domains the shadcn chart examples produce (data min/max, axis defaults),
// then edge cases, then generated ones.
const domains = [
  [0, 0], [0, 1], [0, 10], [0, 100], [0, 186], [0, 214], [0, 237], [0, 305],
  [0, 340], [0, 1000], [0, 1200], [0, 5000], [0, 0.5], [0, 0.03], [-50, 50],
  [-1, 1], [-3.5, 7.25], [12, 13], [99, 101], [1999, 2024], [0.1, 0.3],
  [0.1 + 0.2, 0.7], [1e-7, 3e-7], [1e9, 5e9], [1e15, 1e15 + 10], [-1e6, -2],
  [5, 5], [-5, -5], [0.5, 0.5], [-0.25, -0.25], [1.5, 1.5], [123.456, 123.456],
  [1e-5, 1e-5], [-0, 0], [10, 0], [100, -100], [3, 1],
  [-Infinity, 0], [0, Infinity], [-Infinity, Infinity], [Infinity, 5], [5, -Infinity],
  [NaN, 1], [0, NaN], [-1e308, 1e308], [0, 1e-320], [0, 5e-324], [1e300, 1e301],
];
for (let i = 0; i < 300; i++) {
  const a = value();
  const b = rand() < 0.15 ? 0 : value();
  domains.push(rand() < 0.2 ? [b, a] : [a, b]);
}

const counts = [0, 1, 2, 3, 5, 6, 11];
const cases = [];
for (const domain of domains) {
  for (const tickCount of counts) {
    for (const allowDecimals of [true, false]) {
      for (const mode of ["auto", "snap125"]) {
        if (rand() > 0.35 && domain !== domains[0]) continue;
        cases.push({
          domain: domain.map(num),
          tickCount,
          allowDecimals,
          mode,
          nice: run(() => ticks.getNiceTickValues(domain, tickCount, allowDecimals, mode)),
          fixed: run(() => ticks.getTickValuesFixedDomain(domain, tickCount, allowDecimals, mode)),
        });
      }
    }
  }
}

// getDigitCount, including the f64 neighbours of powers of ten, where
// decimal.js's 20-digit log10 rounds up when converted back to a number.
const digitInputs = [0, 1, 9, 10, 11, 0.1, 0.09, 0.099, 1e-7, 123.456, -42, 5e-324, 1.7976931348623157e308];
for (let k = -12; k <= 21; k++) {
  const p = Number(`1e${k}`);
  const buf = new Float64Array([p]);
  const bits = new BigUint64Array(buf.buffer);
  bits[0] -= 1n;
  const below = buf[0];
  bits[0] += 2n;
  const above = buf[0];
  digitInputs.push(p, below, above, p * (1 - 1e-15), p * (1 - 1e-16));
}
const digitCount = digitInputs.map((v) => ({ value: num(v), digitCount: getDigitCount(v) }));

const output = {
  source: `recharts@${recharts.version} lib/util/scale/getNiceTickValues.js, decimal.js-light@${decimal.version}`,
  generator: "tools/reference/gen_fixtures.mjs",
  digitCount,
  cases,
};
// One entry per line keeps the file small and its diffs readable.
const text =
  "{\n" +
  Object.entries(output)
    .map(([key, value]) =>
      Array.isArray(value)
        ? ` ${JSON.stringify(key)}: [\n${value.map((v) => `  ${JSON.stringify(v)}`).join(",\n")}\n ]`
        : ` ${JSON.stringify(key)}: ${JSON.stringify(value)}`,
    )
    .join(",\n") +
  "\n}\n";
const path = join(fixtures, "nice_ticks.json");

if (process.argv.includes("--check")) {
  const committed = readFileSync(path, "utf8").replace(/\r\n/g, "\n");
  if (committed !== text) {
    console.error(`${path} differs from the pinned packages' output; run node gen_fixtures.mjs`);
    process.exit(1);
  }
  console.log(`${path}: up to date (${cases.length} cases)`);
} else {
  mkdirSync(fixtures, { recursive: true });
  writeFileSync(path, text);
  console.log(`wrote ${path}: ${cases.length} cases, ${digitCount.length} digit counts`);
}
