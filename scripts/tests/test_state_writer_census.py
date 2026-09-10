"""The writer census refuses a family that has stopped matching.

⛔⛔ THIS INSTRUMENT HAS ALREADY GONE BLIND ONCE, AND IT REPORTED THE SMALLER
NUMBER WITHOUT COMPLAINT. On 2026-09-10 its `construct` pattern recognised
construction by a hand-kept list of blessed method names — `{ }`, `(`, `::new`,
`::default`, `::from`. Sealing `GroundItem` behind `#[non_exhaustive]` plus
`at_rest`/`released` dropped OCCURRENCE from 25 write-capable sites to 18 **with
no code removed**, and correcting the pattern surfaced a minting site
(`WorldItem::equipping`) that had never been visible at all.

⇒ A census that goes blind reports a SMALLER, TIDIER population. That is the
reassuring direction, and it is the one nobody double-checks — which is why the
floor is per-FAMILY rather than a total: a total hides one family collapsing to
zero behind the others.
"""

from __future__ import annotations

import importlib.util
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts/measure_state_writers.py"


def _module():
    spec = importlib.util.spec_from_file_location("writers", SCRIPT)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_every_family_of_both_domains_still_has_writers():
    module = _module()
    for domain, families in module.DOMAINS.items():
        rows = module.scan(families)
        for family in families:
            found = [r for r in rows if r[0] == family]
            assert len(found) >= 3, (
                f"`{domain}`/`{family}` has {len(found)} write-capable sites. "
                "Measured 2026-09-10: occurrence 26, custody 10, inventory 16, "
                "checkpoint 9, driver_relation 8, input_projection 24, "
                "body_execution 38, custody_reconciliation 4. A family this small "
                "is a SCAN that stopped matching, not a domain that emptied."
            )


def test_the_construct_rule_is_a_SHAPE_and_not_a_list_of_names():
    """⛔ THE REGRESSION ARM FOR THE BLINDNESS ITSELF.

    Rust's own shape says it: an associated function is `Type::snake_case(`, a
    method is `value.snake_case(`. A pattern that instead names `new`, `default`
    and `from` goes blind the moment a type gains a constructor with any other
    name — which is exactly what sealing one did.
    """
    module = _module()
    construct = dict(module.MECHANISMS)["construct"]
    import re

    pattern = re.compile(construct.format(t="Widget"))
    for spelling in ("Widget {", "Widget(", "Widget::new(", "Widget::at_rest(",
                     "Widget::equipping(", "Widget::released("):
        assert pattern.search(spelling), (
            f"`{spelling}` reads as not-a-construction. The rule must be the "
            "SHAPE, not a list of blessed names — a sealed type's new "
            "constructor is precisely the case a list cannot anticipate."
        )
    for not_construction in ("WidgetFactory {", "MyWidget::new(",):
        assert not pattern.search(not_construction), (
            f"`{not_construction}` counted as constructing a `Widget`"
        )
