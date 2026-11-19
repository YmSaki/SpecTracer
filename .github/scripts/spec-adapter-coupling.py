#!/usr/bin/env python3
"""正本の中で、組込 production adapter を 2 つ以上名指しする条項を報告する。

言語（adapter）を 1 つ足すときに既存の条項を書き換えずに済むためには、
adapter の一覧を 1 か所だけに置き、言語共通の条項が adapter を名指ししないことが要る（#335）。
この script は、その形が保たれているかを確かめる。

使い方: spec-adapter-coupling.py [specification.json のパス]
終了コード: 0 = 許容した条項だけ。1 = それ以外に、2 つ以上の adapter を名指しする条項がある。
"""
import json
import re
import sys

PATH = sys.argv[1] if len(sys.argv) > 1 else "docs/canonical/specification.json"
LAYERS = ("require", "spec", "detailed_spec", "basic_design", "design")
# adapter の一覧を置く節（要件定義 §21）と、そこで adapter の対象を定める条項の形。
SCOPE_SECTION = "REQ-S048"
SCOPE_FORM = re.compile(r"^`([a-z][a-z0-9-]*)` は .*を対象とする。$")
# 2 つの adapter を主語に持つ条項。新しい adapter は自分の条項を足せばよく、書き換えは要らない。
ALLOWED = {"DS-1984", "SPEC-490"}


def walk(section):
    yield section
    for child in section.get("sections", []):
        yield from walk(child)


def main():
    with open(PATH, encoding="utf-8") as handle:
        document = json.load(handle)
    sections = [s for layer in LAYERS for top in document[layer] for s in walk(top)]
    adapters = []
    for section in sections:
        if section["id"] == SCOPE_SECTION:
            for item in section.get("items", []):
                match = SCOPE_FORM.match(item["statement"])
                if match:
                    adapters.append(match.group(1))
    if not adapters:
        print(f"{SCOPE_SECTION} に、adapter の対象を定める条項が見つからない")
        return 1
    names = [re.compile(r"`?" + re.escape(a) + r"`?" if "-" in a else r"`" + re.escape(a) + r"`") for a in adapters]
    found = []
    for section in sections:
        for item in section.get("items", []):
            named = [a for a, pattern in zip(adapters, names) if pattern.search(item["statement"])]
            if len(named) >= 2:
                found.append((item["id"], named))
    unexpected = [(i, n) for i, n in found if i not in ALLOWED]
    print(f"組込 production adapter: {', '.join(adapters)}")
    print(f"adapter を 2 つ以上名指しする条項: {len(found)} 件（許容 {len(found) - len(unexpected)} 件）")
    for clause, named in unexpected:
        print(f"  {clause}: {', '.join(named)}")
    return 1 if unexpected else 0


if __name__ == "__main__":
    sys.exit(main())
