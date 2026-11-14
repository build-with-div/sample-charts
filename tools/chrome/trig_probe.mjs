// Records Chromium's Math.sin, Math.cos, Math.tan and Math.asin on a fixed
// set of arguments: serves a page that evaluates them, loads it in headless
// Chrome and writes crates/js/tests/fixtures/chrome_trig.json. Then run
// trig_gaps.py on it to record which sin and cos results are not the
// correctly rounded ones.
//
//   node tools/chrome/trig_probe.mjs <path to chrome or chrome.exe>
//
// The arguments: 6,000 radians in [-2π, 2π]; angles in degrees as Recharts
// converts them (-RADIAN * angle), 3,000 with two decimals in [-360, 360]
// and 1,500 whole; 2,000 small and large values (10^-20 to 10^19); 500 huge
// values (up to 10^299); the arguments of d3's star symbol; and those the
// fixture generators call Math.sin and Math.cos with, which
// tools/reference/chrome_trig.mjs lists in trig_args.json. Repeated
// arguments are recorded once. Math.asin takes 6,000 arguments in [-1, 1],
// 2,000 small ones and the boundaries of its branches.
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const chrome = process.argv[2];
if (!chrome) {
  console.error("usage: node trig_probe.mjs <chrome executable>");
  process.exit(2);
}
const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "../../crates/js/tests/fixtures/chrome_trig.json");
const fixtureArgs = join(here, "trig_args.json");

let seed = 0x7419;
const rand = () => ((seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0) / 2 ** 32);
const RADIAN = Math.PI / 180;
const inputs = [];
for (let i = 0; i < 6000; i++) inputs.push((rand() * 2 - 1) * 2 * Math.PI);
for (let i = 0; i < 3000; i++) inputs.push(-RADIAN * (Math.round(rand() * 72000) / 100 - 360));
for (let i = 0; i < 1500; i++) inputs.push(-RADIAN * Math.round(rand() * 720 - 360));
for (let i = 0; i < 2000; i++) inputs.push((rand() * 2 - 1) * 10 ** Math.floor(rand() * 40 - 20));
for (let i = 0; i < 500; i++) inputs.push((rand() * 2 - 1) * 10 ** Math.floor(rand() * 300));
inputs.push(Math.PI / 10, (7 * Math.PI) / 10, (2 * Math.PI) / 10, 18 * RADIAN, 36 * RADIAN);
for (const i of [1, 2, 3, 4]) inputs.push((2 * Math.PI * i) / 5);

// String(-0) is "0"; keep the sign.
const enc = (v) => (Object.is(v, -0) ? "-0" : String(v));
const extra = existsSync(fixtureArgs) ? JSON.parse(readFileSync(fixtureArgs, "utf8")) : [];
const args = [...new Set([...inputs.map(enc), ...extra])];
const unit = [];
for (let i = 0; i < 6000; i++) unit.push(rand() * 2 - 1);
for (let i = 0; i < 2000; i++) unit.push((rand() * 2 - 1) / 10 ** Math.floor(rand() * 10));
unit.push(-0, 0.5, -0.5, 0.975, -0.975, 0.9749999999999999, 1, -1, 1.0000000000000002, 2 ** -27, -(2 ** -28));
const asinArgs = [...new Set(unit.map(enc))];

const page = `<!doctype html><script>
const args = ${JSON.stringify(args)};
const asinArgs = ${JSON.stringify(asinArgs)};
const enc = ${enc};
const result = {
  userAgent: navigator.userAgent,
  sin: args.map((x) => enc(Math.sin(Number(x)))),
  cos: args.map((x) => enc(Math.cos(Number(x)))),
  tan: args.map((x) => enc(Math.tan(Number(x)))),
  asin: asinArgs.map((x) => enc(Math.asin(Number(x)))),
};
fetch("/result", { method: "POST", body: JSON.stringify(result) });
</script>`;

const profile = mkdtempSync(join(tmpdir(), "trig-probe-"));
let browser;
const server = createServer((req, res) => {
  if (req.method !== "POST") {
    res.setHeader("content-type", "text/html");
    res.end(page);
    return;
  }
  let body = "";
  req.on("data", (chunk) => (body += chunk));
  req.on("end", () => {
    res.end("ok");
    const result = JSON.parse(body);
    const lines = args.map((x, i) => `  ${JSON.stringify([x, result.sin[i], result.cos[i], result.tan[i]])}`);
    const asinLines = asinArgs.map((x, i) => `  ${JSON.stringify([x, result.asin[i]])}`);
    writeFileSync(
      out,
      `{\n "browser": ${JSON.stringify(result.userAgent)},\n "columns": ["x", "sin", "cos", "tan"],\n` +
        ` "cases": [\n${lines.join(",\n")}\n ],\n "asin": [\n${asinLines.join(",\n")}\n ]\n}\n`,
    );
    console.log(`${args.length} sin/cos/tan and ${asinArgs.length} asin arguments from ${result.userAgent}`);
    browser.kill();
    server.close();
  });
});
server.listen(0, "127.0.0.1", () => {
  const url = `http://127.0.0.1:${server.address().port}/`;
  browser = spawn(chrome, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    `--user-data-dir=${profile}`,
    url,
  ]);
  browser.on("exit", () => {
    try {
      rmSync(profile, { recursive: true, force: true });
    } catch {
      // Chrome may still hold files in the profile briefly.
    }
  });
});
