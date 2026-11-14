# Framework gaps

The web platform features the ported chart packages need and Div (pinned
in `Cargo.toml`) lacks, as they are hit while porting. Each one is fixed in
Div by implementing that feature with Chromium's behaviour, so the ported
package runs unchanged; the sample never ships its own version of a
platform feature.

Where the sample has to keep building without a feature, the code carries
temporary scaffolding marked `// div-gap: <id> <web feature>`, findable
with `git grep -n "div-gap: V1"`. Scaffolding is never the solution and is
removed when Div gains the feature.

IDs are the platform plan's (`consolidated-plan.md` of the Div platform
work); the "Work item" column is the row of the
[design's work list](design.md#the-div-work-list). Owners: **engine**
(div-dom, div-style, div-layout, div-paint, div-text, div-engine),
**framework** (divui, div-platform, div-macros). Sizes: S (days), M (one
to two weeks), L (more). Owners and scheduling follow the platform
decisions accepted on 13 November 2026
(`platform-plan/decisions-2026-11-13.md`).

Pinned Div: `92ea25a0`.

## Hit in the code

| ID | Web feature | Work item | Owner | Size | Where | Scaffolding |
| --- | --- | --- | --- | --- | --- | --- |
| V1 | Inline SVG core: `svg` sizing and `viewBox`, `g`, `path`, shapes, fill and stroke, `transform`, painted as Chromium paints SVG; `view!` with SVG's attribute names. Scheduled right after the engine's Tier 0 items | 1, 13 | engine (painting), framework (`view!`) | L | `src/ui.rs` | None: the window shows the ported tick values as text instead of charts. No SVG is drawn by other means. |

## Expected next, from the design

Not yet hit in code; listed so the order of the port is visible. Each moves
to the table above, with its `div-gap` site, when a milestone reaches it.

| ID | Web feature | Work item | Owner | Size | Milestone |
| --- | --- | --- | --- | --- | --- |
| V2 | SVG text: `text`, `tspan`, anchors, baselines | 2 | engine | L | M4 (axis labels) |
| S1, S2 | Selectors, `<style>` sheets, SVG properties in the cascade; shadcn's `ChartStyle` through the browser-compatible ingestion mode | 3 | engine | L | M4, M5 |
| S3 | CSS custom properties, `var()` in SVG paint, runtime writes (`setProperty('--x')`) | 4 | engine (framework: runtime API) | M (L with `@property`) | M5 (shadcn colours) |
| O1 | `ResizeObserver`: one API, three boxes; engine computes after layout, framework delivers before paint | 5 | engine + framework | M | M4 (`ResponsiveContainer`) |
| R1 | `requestAnimationFrame` per document, typed handle, every window | 6 | framework | M | M6 (mount animation) |
| V5 | Focus, `tabindex`, roles on SVG; hit testing on SVG geometry, `pointer-events` | 7, 10 | engine + framework | M | M7 |
| V3 | `defs`, gradients, `pattern`, `clipPath`, `url(#id)` | 8 | engine | M | M6 |
| V4 | `getBBox`, `getTotalLength`, `getPointAtLength`, `getComputedTextLength` | 9 | framework (engine geometry) | M | M4 (Recharts measures text), M6 |
| A1 | CSS transitions on SVG properties, through the one Web Animations surface | 11 | engine (framework: Web Animations API) | L | M6 |
| I2 | `contextmenu`; right click must not fire `click` | 14 | framework | S–M | M7 |
| R7 | Per-element build, layout and paint cost | 15 | engine + framework | L | M6 (to measure against Chromium) |
| C1–C4 | `HTMLCanvasElement`, `CanvasRenderingContext2D`, `Path2D`, `measureText` | 12 | engine (`div-canvas`) | L | M8 (Chart.js) |
| H1 | Releasable image resources | 17 | framework + engine | M | Only if scaffolding needs it |

## Not Div's: the ported package's own behaviour

Found while porting; the port keeps it, because it is what Recharts does
in the browser.

- `getNiceTickValues` with fewer than three ticks for an interval that
  strictly contains 0 never settles: `calculateStep` recurses until V8
  throws `Maximum call stack size exceeded` (or, with `niceTicks="snap125"`
  and integer ticks, until the step overflows and decimal.js-light throws
  `Invalid argument: Infinity`). The port returns the same errors. When the
  overflow happens inside decimal.js-light, the page's copy of the library
  can be left with changed internal state that alters later results; the
  port does not reproduce that.
- decimal.js-light truncates arithmetic to 20 significant digits rather
  than rounding, and `getDigitCount` of the largest doubles below a power
  of ten counts one digit more (`getDigitCount(999.9999999999999)` is 4).
  The port matches both (`crates/recharts/src/decimal.rs`,
  `scale/arithmetic.rs`).
- Recharts ships two builds that use different d3 code. The browser build
  (`es6`, what bundlers pick) imports `victory-vendor/d3-scale`, which
  re-exports npm d3-scale 4.0.3 and d3-array 3.2.5; the CommonJS build
  (`lib`) uses older copies vendored inside victory-vendor, whose `ticks`
  computes the step differently and gives other ticks for some domains. The
  port follows the browser build (`crates/d3`).
- `scale="implicit"` passes Recharts' scale-name check (`scaleImplicit` is
  a d3-scale export) but is d3's unknown-value symbol, not a constructor,
  so building the axis scale throws a TypeError. The port throws it with
  the message V8 gives for the unminified source, `d3Scales[name] is not a
  function`; a minifying bundler renames `d3Scales`, which changes it.
- Chromium's own `Math.pow` depends on the platform: its V8 calls the C
  library's `pow`. Measured on Windows 11 with Chrome for Testing
  147.0.7727.50: `Math.pow(10, 23)` is 1.0000000000000001e23, not the
  correctly rounded 1e23 (Chromium on Linux was not measured). Rust's
  `f64::powf` on the same
  machine also rounds 10^126 up where Chrome does not, so the port matches
  Chromium on 683 of the 684 calls checked. Axis ticks only reach these
  results for values above about 1e22, which the fixtures avoid;
  `crates/js/src/math.rs` records the measurement.
- Recharts memoizes `selectNumericalDomain`, `selectDomainOfStackGroups`
  and the data-domain selector under them with `numberDomainEqualityCheck`
  (reselect 5.1.1's `resultEqualityCheck`): when a new domain is `===`
  equal to the selector's previous result, the previous array is returned
  instead. `0 === -0`, so an axis whose domain starts at `-0` can come back
  with `0` (or the reverse) depending on which axis the page selected
  before it, and reselect keeps that previous result in a `WeakRef`, so
  also on garbage collection. Found in the fixture generator and replayed
  in Node 24.22.3: in one generated chart, selecting the X axis first
  (`selectNumericalDomain` gives `[-0, 648514000000]`) makes the Y axis
  return that array, where the Y axis selected alone gives
  `[0, 648514000000]`. Only the sign of a zero
  domain bound changes. The port computes each axis on its own, and
  `axis_fixtures.mjs` resets the three selectors' last result before every
  axis so the fixtures do the same.
- Chromium 147 does not compute `Math.sin` and `Math.cos` as Node 24
  does. Measured on Windows 11, Chrome for Testing 147.0.7727.50
  (headless) against Node 24.22.3 on the same 13,009 arguments
  (angles in radians and degrees, small, large and huge values): 397 `sin`
  and 396 `cos` results differ, among them `Math.cos(2 * Math.PI * 3 / 5)`,
  which d3's star symbol uses (-0.8090169943749476 in Chrome,
  -0.8090169943749475 in Node). Chrome's results also differ from the
  Windows C runtime's (390 `sin`, 399 `cos`), so they come from V8's
  bundled `sin`/`cos`, not from the platform. They are the correctly
  rounded values on all but 23 `sin` and 10 `cos` results (18 and 10
  distinct arguments), each a near tie (the exact value within 0.013 ULP
  of halfway between two doubles) that Chrome rounds 1 ULP the other way;
  among them `Math.sin` of -342° and 342° in radians. Every argument of
  d3's star symbol is correctly rounded in Chrome. `Math.tan`, `Math.asin`, `Math.acos`,
  `Math.atan`, `Math.atan2` and `Math.sqrt` agreed with Node on every
  argument tried (V8's fdlibm port in both). Node-generated fixtures are
  therefore not a reference for anything drawn with `sin` or `cos`
  (`<Sector>`, the star symbol, polar layout). V8's `sin`/`cos` are not
  ported (licence); `crates/js` computes correctly rounded `sin`/`cos` with
  code of its own (M3d). Remaining gap: measured with
  `tools/chrome/trig_probe.mjs` on 15,484 distinct arguments (the probe's
  12,046 plus the 3,438 the shape fixtures use), the port equals Chrome
  except on 28 `sin` and 13 `cos` results, the ones Chrome does not round
  correctly (`crates/js/tests/fixtures/chrome_trig_gaps.json`, checked by
  `cargo test -p js --test trig`). Fixtures that depend on `sin`/`cos`
  take Chrome's values, not Node's: `tools/reference/chrome_trig.mjs`
  substitutes the recorded results while the packages run (M3e). 13 of
  the gaps are arguments of the `<Sector>` fixtures; none changes a path
  after Recharts' 4-digit rounding.
