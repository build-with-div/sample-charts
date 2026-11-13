#!/usr/bin/env python3
"""Render the fidelity probe's SVG pages in pinned Chromium and compare them
with the Div renders, pixel for pixel.

Run on Linux (WSL here) after `cargo test --release --test fidelity_sample`:

    python3 compare.py --div-checkout PATH_TO_DIV_565db46 [--out out/fidelity]

Chromium is launched by Div's own `run_all_pixel_comparisons.render_chrome`
(the components harness path: headless, --disable-gpu, CDP device metrics)
at 400x200 CSS px, scale 1 and 2. Pages hold geometry only (no text), so the
font environment does not matter.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from PIL import Image, ImageChops

DEFAULT_CHROME = Path.home() / "openui-oracle/chrome/linux-147.0.7727.50/chrome-linux64/chrome"


def compare(expected: Path, actual: Path) -> dict:
    with Image.open(expected) as e, Image.open(actual) as a:
        e, a = e.convert("RGB"), a.convert("RGB")
        if e.size != a.size:
            return {"status": "size", "chromium": e.size, "div": a.size}
        diff = ImageChops.difference(e, a)
        channels = diff.split()
        mask = channels[0]
        for channel in channels[1:]:
            mask = ImageChops.lighter(mask, channel)
        histogram = mask.histogram()
        mismatched = sum(histogram[1:])
        result = {"pixels": e.size[0] * e.size[1], "mismatched": mismatched}
        if mismatched:
            result["max_delta"] = max(c.getextrema()[1] for c in channels)
            result["over_8"] = sum(histogram[9:])
            result["bbox"] = list(mask.getbbox())
        return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--div-checkout", type=Path, required=True)
    parser.add_argument("--chrome", type=Path, default=DEFAULT_CHROME)
    parser.add_argument("--out", type=Path, default=Path(__file__).parent / "out/fidelity")
    args = parser.parse_args()
    sys.path.insert(0, str(args.div_checkout / "tools/accountability"))
    import run_all_pixel_comparisons as pixel_runner  # noqa: E402

    results = {}
    for page in sorted(args.out.glob("*.html")):
        for scale in (1, 2):
            chromium = args.out / f"{page.stem}@{scale}x-chromium.png"
            ok = pixel_runner.render_chrome(
                str(page.resolve()), str(chromium), str(args.chrome), str(args.chrome.parent),
                logical_width=400, logical_height=200, device_scale=float(scale),
            )
            key = f"{page.stem}@{scale}x"
            if not ok:
                results[key] = {"status": "chromium failed"}
                continue
            for variant in ("div", "div-opaque", "raw"):
                results[f"{key} {variant}"] = compare(
                    chromium, args.out / f"{page.stem}@{scale}x-{variant}.png")
        for scale in (1, 2):
            results[f"{page.stem}@{scale}x raw-vs-div-opaque"] = compare(
                args.out / f"{page.stem}@{scale}x-raw.png",
                args.out / f"{page.stem}@{scale}x-div-opaque.png")
    for key, r in results.items():
        print(f"| {key} | {r.get('mismatched', r.get('status'))} | {r.get('max_delta', 0)} | {r.get('over_8', 0)} |")
    (args.out / "results.json").write_text(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
