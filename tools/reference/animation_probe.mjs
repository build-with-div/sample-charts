// Which browser APIs Recharts' entrance animations use: renders shadcn's
// default area, line and bar charts with their animations on (Recharts'
// default, isAnimationActive "auto") in headless Chrome, with the pinned
// react, react-dom and recharts bundled by esbuild as dom_fixtures.mjs
// does, and counts the calls of every timing, media, animation and SVG
// geometry API a page could animate with, from before the bundle loads
// until the chart has settled.
//
//   node animation_probe.mjs <path to chrome or chrome.exe>
//
// It prints one JSON object per chart: the calls, the CSS transitions and
// animations left (document.getAnimations()), the matchMedia queries, and
// the area's animation clip rectangle sampled on each frame.

import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

// Installed before the bundle runs, so every call is seen.
const instrument = `
// The probe's own frame loop and checks use the originals in __raw.
window.__raw = { requestAnimationFrame: window.requestAnimationFrame.bind(window), now: performance.now.bind(performance), getAnimations: document.getAnimations.bind(document) };
window.__calls = {};
const count = (name) => (window.__calls[name] = (window.__calls[name] || 0) + 1);
window.__queries = [];
const wrap = (owner, name, label, after) => {
  const original = owner[name];
  if (typeof original !== "function") return;
  owner[name] = function () {
    count(label);
    const result = original.apply(this, arguments);
    if (after) after(arguments, result);
    return result;
  };
};
wrap(window, "requestAnimationFrame", "requestAnimationFrame");
wrap(window, "cancelAnimationFrame", "cancelAnimationFrame");
wrap(window, "setTimeout", "setTimeout");
wrap(window, "setInterval", "setInterval");
wrap(window, "matchMedia", "matchMedia", (args) => window.__queries.push(String(args[0])));
wrap(performance, "now", "performance.now");
wrap(Date, "now", "Date.now");
wrap(Element.prototype, "animate", "Element.animate");
wrap(Document.prototype, "getAnimations", "document.getAnimations");
wrap(SVGGeometryElement.prototype, "getTotalLength", "getTotalLength");
wrap(SVGGeometryElement.prototype, "getPointAtLength", "getPointAtLength");
wrap(SVGGraphicsElement.prototype, "getBBox", "getBBox");
wrap(CSSStyleDeclaration.prototype, "setProperty", "style.setProperty");
wrap(MediaQueryList.prototype, "addEventListener", "MediaQueryList.addEventListener");
wrap(MediaQueryList.prototype, "addListener", "MediaQueryList.addListener");
`;

const entry = `
import React from "react";
import { createRoot } from "react-dom/client";
import { AreaChart, Area, LineChart, Line, BarChart, Bar, CartesianGrid, XAxis } from "recharts";
const data = [
  { month: "January", desktop: 186 },
  { month: "February", desktop: 305 },
  { month: "March", desktop: 237 },
  { month: "April", desktop: 73 },
  { month: "May", desktop: 209 },
  { month: "June", desktop: 214 },
];
const axis = { dataKey: "month", tickLine: false, axisLine: false, tickMargin: 8 };
const charts = {
  area: [AreaChart, { margin: { left: 12, right: 12 } }, React.createElement(Area, { dataKey: "desktop", type: "natural", fill: "var(--color-desktop)", fillOpacity: 0.4, stroke: "var(--color-desktop)" })],
  line: [LineChart, { margin: { left: 12, right: 12 } }, React.createElement(Line, { dataKey: "desktop", type: "natural", stroke: "var(--color-desktop)", strokeWidth: 2, dot: false })],
  bar: [BarChart, {}, React.createElement(Bar, { dataKey: "desktop", fill: "var(--color-desktop)", radius: 8 })],
};
window.renderChart = (name) => new Promise((resolve) => {
  window.__calls = {};
  window.__queries = [];
  const [Chart, props, item] = charts[name];
  const container = document.getElementById("root");
  const root = createRoot(container);
  const raw = window.__raw;
  const start = raw.now();
  const clip = [];
  let last = null;
  let same = 0;
  const frame = (time) => {
    const rect = container.querySelector("clipPath[id^=animationClipPath] rect");
    if (rect) clip.push([Math.round(time - start), rect.getAttribute("width")]);
    const html = container.innerHTML;
    same = html === last ? same + 1 : 0;
    last = html;
    if (same < 30 && time - start < 5000) raw.requestAnimationFrame(frame);
    else {
      const animations = raw.getAnimations().length;
      const transitions = Array.from(container.querySelectorAll("*"))
        .map((el) => getComputedStyle(el).transitionProperty)
        .filter((p) => p && p !== "all" && p !== "none");
      const calls = { ...window.__calls };
      root.unmount();
      resolve({ chart: name, ms: Math.round(time - start), calls, queries: window.__queries, animations, transitions: [...new Set(transitions)], clip: clip.filter((_, i) => i % 15 === 0 || i === clip.length - 1) });
    }
  };
  root.render(React.createElement(Chart, { width: 500, height: 300, accessibilityLayer: true, data, ...props }, React.createElement(CartesianGrid, { vertical: false }), React.createElement(XAxis, axis), item));
  raw.requestAnimationFrame(frame);
});
`;

async function main() {
  const chrome = process.argv[2];
  if (!chrome) {
    console.error("usage: node animation_probe.mjs <chrome executable>");
    process.exit(2);
  }
  const esbuild = createRequire(join(here, "package.json"))("esbuild");
  const out = await esbuild.build({
    stdin: { contents: entry, resolveDir: here, loader: "js" },
    bundle: true,
    write: false,
    format: "iife",
    platform: "browser",
    define: { "process.env.NODE_ENV": '"production"' },
    logLevel: "error",
  });
  const page = `<!doctype html><html><head><meta charset="utf-8"><script>${instrument}</script></head><body><div id="root"></div>
<script src="/bundle.js"></script><script>
(async () => {
  const results = [];
  for (const name of ["area", "line", "bar"]) results.push(await window.renderChart(name));
  await fetch("/result", { method: "POST", body: JSON.stringify({ userAgent: navigator.userAgent, results }) });
})().catch((e) => fetch("/result", { method: "POST", body: JSON.stringify({ failure: String(e && e.stack || e) }) }));
</script></body></html>`;
  const profile = mkdtempSync(join(tmpdir(), "animation-probe-"));
  let browser;
  const result = await new Promise((resolve, reject) => {
    const server = createServer((req, res) => {
      if (req.method === "POST") {
        let body = "";
        req.on("data", (chunk) => (body += chunk));
        req.on("end", () => {
          res.end("ok");
          browser.kill();
          server.close();
          const parsed = JSON.parse(body);
          if (parsed.failure) reject(new Error(parsed.failure));
          else resolve(parsed);
        });
        return;
      }
      res.setHeader("content-type", req.url === "/bundle.js" ? "text/javascript" : "text/html");
      res.end(req.url === "/bundle.js" ? out.outputFiles[0].text : page);
    });
    server.listen(0, "127.0.0.1", () => {
      browser = spawn(chrome, [
        "--headless=new",
        "--disable-gpu",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-background-timer-throttling",
        "--disable-renderer-backgrounding",
        `--user-data-dir=${profile}`,
        `http://127.0.0.1:${server.address().port}/`,
      ]);
    });
  });
  try {
    rmSync(profile, { recursive: true, force: true });
  } catch {
    // Chrome may still hold files in the profile briefly.
  }
  console.log(result.userAgent);
  for (const r of result.results) console.log(JSON.stringify(r));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
