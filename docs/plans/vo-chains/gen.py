#!/usr/bin/env python3
# VO トレーサビリティ・チェーン生成器。
# data.yaml（ノード + リンク + 意味）を辿り、階層リストのチェーン表示 + ID 逆引き Index +
# 完全性チェックリストを Markdown で吐く。表示は手書きせずこの生成器が出す（P-003: 単一の正典）。
# 将来 SpecTracer 本体のトレース機能が置き換える、そのプロトタイプ。
#
# 使い方: python gen.py <data.yaml> <out.md>

import sys, yaml

# 各ノード種別の見出し表記。id をインラインに畳む（種別（id）— 意味）。
def node_label(n):
    t = n["type"]
    m = n.get("meaning", "")
    if t in ("要求", "原則"):
        idpart = n.get("id", "")
        label = n.get("label")
        inner = f"{idpart}・{label}" if label else idpart
        return f"{t}（{inner}）— {m}"
    if t == "項目":
        return f"**項目: {m}**"
    if t == "実装":
        path = n.get("path", "")
        status = n.get("status", "")
        tail = f" `{path}`" if path else ""
        tail += f"（現状: {status}）" if status else ""
        return f"**実装** — {m}{tail}"
    if t == "テスト実装":
        status = n.get("status", "")
        return f"**テスト実装** — 現状: {status}"
    # 要件 / 基本仕様 / 詳細仕様 / VO / テスト
    idpart = n.get("id")
    head = f"**{t}（{idpart}）**" if idpart else f"**{t}**"
    return f"{head} — {m}"

def render_children(n, depth, out):
    # depth: この node の子を出すインデント段（子は depth 段字下げ）
    for c in n.get("children", []):
        indent = "  " * depth
        out.append(f"{indent}- {node_label(c)}")
        # 項目の子（実装 / VO）の前で少し読みやすく空行は入れない（詰めて表示）
        render_children(c, depth + 1, out)

def collect_ids(n, chain_title, index):
    # id を持つノードを Index に集める（どの要求チェーンに属すか）
    idv = n.get("id")
    if idv:
        index.setdefault(idv, set()).add(chain_title)
    for c in n.get("children", []):
        collect_ids(c, chain_title, index)

def collect_gaps(n, path, gaps):
    # 完全性チェック: 項目ごとに「実装」と「テスト実装」の有無を見る。
    if n.get("type") == "項目":
        impl = next((c for c in n.get("children", []) if c["type"] == "実装"), None)
        vo = next((c for c in n.get("children", []) if c["type"] == "VO"), None)
        test = None; test_impl = None
        if vo:
            test = next((c for c in vo.get("children", []) if c["type"] == "テスト"), None)
            if test:
                test_impl = next((c for c in test.get("children", []) if c["type"] == "テスト実装"), None)
        def present(node):
            # status に「未」を含む、または node 自体が無ければ「無し」
            if node is None:
                return False
            st = node.get("status", "")
            return "未" not in st
        gaps.append({
            "item": n.get("meaning", ""),
            "vo": (vo or {}).get("id", "—"),
            "impl": present(impl),
            "test": present(test_impl),
        })
    for c in n.get("children", []):
        collect_gaps(c, path, gaps)

def main():
    src, dst = sys.argv[1], sys.argv[2]
    with open(src, encoding="utf-8") as f:
        data = yaml.safe_load(f)
    out = []
    out.append("# VO トレーサビリティ・チェーン")
    out.append("")
    index = {}
    gaps = []
    for root in data["roots"]:
        title = node_label(root)
        out.append(f"## {title}")
        out.append("")
        render_children(root, 1, out)
        out.append("")
        collect_ids(root, root.get("id", title), index)
        collect_gaps(root, [], gaps)

    # ID 逆引き Index
    out.append("---")
    out.append("")
    out.append("## Index（ID → ルート）")
    out.append("")
    for idv in sorted(index):
        chains = "、".join(sorted(index[idv]))
        out.append(f"- `{idv}` → {chains}")
    out.append("")

    # 完全性チェック（項目 × 実装/テスト実装）
    out.append("## 完全性チェック（項目ごとの実装・検証の有無）")
    out.append("")
    out.append("| 項目 | VO | 実装 | テスト実装 | 状態 |")
    out.append("|---|---|---|---|---|")
    for g in gaps:
        impl = "○" if g["impl"] else "×"
        test = "○" if g["test"] else "×"
        if g["impl"] and g["test"]:
            state = "検証済み"
        elif g["impl"] and not g["test"]:
            state = "作ったが未検証"
        elif not g["impl"] and g["test"]:
            state = "仕様とテストはあるが未実装"
        else:
            state = "実装・テストとも未"
        out.append(f"| {g['item']} | {g['vo']} | {impl} | {test} | {state} |")
    out.append("")

    with open(dst, "w", encoding="utf-8") as f:
        f.write("\n".join(out))
    print(f"生成: {dst}  （要求 {len(data['roots'])} / 項目 {len(gaps)}）")

if __name__ == "__main__":
    main()
