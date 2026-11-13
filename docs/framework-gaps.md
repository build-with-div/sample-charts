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
