#!/usr/bin/env python3
"""Rank the platform features shadcn's chart examples need.

Fetch the examples first (70 files at shadcn@4.23.0):

    mkdir -p out/shadcn-charts && cd out/shadcn-charts
    for f in $(gh api "repos/shadcn-ui/ui/contents/apps/v4/registry/new-york-v4/charts?ref=shadcn@4.23.0" \
                 --jq '.[].name' | grep '^chart-'); do
      gh api "repos/shadcn-ui/ui/contents/apps/v4/registry/new-york-v4/charts/$f?ref=shadcn@4.23.0" \
        -H "Accept: application/vnd.github.raw" > "$f"
    done

then `python3 count_examples.py out/shadcn-charts`. The rules map Recharts
3.8.0 components and props to what it renders; see docs/design.md, appendix.
"""

import re
import sys
from pathlib import Path


def has(source: str, *patterns: str) -> bool:
    return any(re.search(p, source) for p in patterns)


FEATURES = {
    "svg root, g layers, path": lambda s: True,
    "CSS stylesheet cascade on SVG": lambda s: True,
    "ResizeObserver": lambda s: True,
    "focusable svg (accessibilityLayer default)": lambda s: True,
    "CSS custom properties in SVG paint": lambda s: "var(--color-" in s,
    "mouse events on wrapper (tooltip)": lambda s: "ChartTooltip" in s,
    "text/tspan": lambda s: has(s, r"<XAxis", r"<YAxis", r"<PolarAngleAxis",
                                r"<PolarRadiusAxis", r"<Label\b", r"<LabelList", r"<text"),
    "line": lambda s: has(s, r"<CartesianGrid", r"<PolarGrid")
    or (has(s, r"<XAxis", r"<YAxis") and "tickLine={false}" not in s),
    "path arcs (A)": lambda s: has(s, r"<Pie\b", r"<RadialBar\b")
    or (has(s, r"<Bar\b") and "radius=" in s),
    "circle": lambda s: has(s, r"<Line\b", r"<Area\b", r"dot=\{", r'gridType="circle"', r"activeDot"),
    "fill-opacity / stroke-opacity": lambda s: has(s, "fillOpacity", "strokeOpacity"),
    "cubic Beziers (C)": lambda s: has(s, r'type="natural"', r'type="monotone"'),
    "stroke-dasharray": lambda s: has(s, "strokeDasharray", r"<Line\b"),
    "getTotalLength": lambda s: has(s, r"<Line\b"),
    "clipPath": lambda s: has(s, r"<Area\b"),
    "hit testing on SVG shapes": lambda s: has(s, r"<PieChart") and "ChartTooltip" in s,
    "linearGradient": lambda s: "linearGradient" in s,
}


def main() -> None:
    examples = {p.name: p.read_text(encoding="utf-8") for p in Path(sys.argv[1]).glob("chart-*.tsx")}
    print(f"examples: {len(examples)}")
    rows = sorted(((sum(fn(s) for s in examples.values()), name) for name, fn in FEATURES.items()),
                  reverse=True)
    for count, name in rows:
        print(f"| {name} | {count} |")


if __name__ == "__main__":
    main()
