"""Finds the Chromium Math.sin/Math.cos results that are not correctly rounded.

Reads crates/js/tests/fixtures/chrome_trig.json (written by trig_probe.mjs),
computes each correctly rounded result with Python's decimal module (pi from
Machin's formula to 500 digits, an exact reduction, then the Taylor series
at 80 digits), and writes the cases where Chrome differs, with the position
of the exact value between the two doubles, to chrome_trig_gaps.json.

    python -I tools/chrome/trig_gaps.py
"""

import json
import math
from decimal import Decimal, getcontext
from pathlib import Path

FIXTURES = Path(__file__).resolve().parents[2] / "crates" / "js" / "tests" / "fixtures"

getcontext().prec = 500


def atan_inv(n):
    x = Decimal(1) / n
    x2 = x * x
    term, total, k = x, x, 1
    eps = Decimal(10) ** -(getcontext().prec + 5)
    while abs(term) > eps:
        term *= -x2
        total += term / (2 * k + 1)
        k += 1
    return total


PI = 16 * atan_inv(5) - 4 * atan_inv(239)
TWO_PI = 2 * PI


def sin_cos(x):
    """The sine and cosine of the double x, to about 75 digits."""
    x = Decimal(x)
    r = x - TWO_PI * (x / TWO_PI).to_integral_value()
    getcontext().prec = 80
    r = +r
    eps = Decimal(10) ** -75
    s, term, n = Decimal(0), r, 1
    while abs(term) > eps:
        s += term
        term = -term * r * r / ((n + 1) * (n + 2))
        n += 2
    c, term, n = Decimal(0), Decimal(1), 0
    while abs(term) > eps:
        c += term
        term = -term * r * r / ((n + 1) * (n + 2))
        n += 2
    getcontext().prec = 500
    return s, c


def parse(text):
    return -0.0 if text == "-0" else float(text)


def js_string(v):
    """Number::toString for the values here (none is an integer above 2^53)."""
    if v == 0:
        return "-0" if math.copysign(1, v) < 0 else "0"
    if v == int(v) and abs(v) < 1e21:
        return str(int(v))
    r = repr(v)
    if "e" in r:
        mantissa, exponent = r.split("e")
        exponent = int(exponent)
        return f"{mantissa}e{'+' if exponent > 0 else '-'}{abs(exponent)}"
    return r


def main():
    cases = json.loads((FIXTURES / "chrome_trig.json").read_text())["cases"]
    gaps = []
    for x_text, sin_text, cos_text, *_ in cases:
        x = parse(x_text)
        exact = (Decimal(x), Decimal(1)) if x == 0 else sin_cos(x)
        for name, value, chrome_text in (("sin", exact[0], sin_text), ("cos", exact[1], cos_text)):
            rounded = float(value)
            if x == 0 and name == "sin":
                rounded = x
            if rounded != parse(chrome_text):
                position = (value - Decimal(rounded)) / Decimal(math.ulp(rounded))
                gaps.append(
                    {
                        "fn": name,
                        "x": x_text,
                        "chrome": chrome_text,
                        "correctlyRounded": js_string(rounded),
                        "exactMinusCorrectlyRoundedUlp": f"{float(position):+.4f}",
                    }
                )
    lines = ",\n".join("  " + json.dumps(g) for g in gaps)
    (FIXTURES / "chrome_trig_gaps.json").write_text(f"[\n{lines}\n]\n", newline="\n")
    for name in ("sin", "cos"):
        print(f"{name}: {sum(g['fn'] == name for g in gaps)} of {len(cases)} not correctly rounded")


if __name__ == "__main__":
    main()
