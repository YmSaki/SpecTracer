#!/usr/bin/env python3
# VO 裁定台帳ジェネレータ。
# ledger.yaml（4監査を dedup した負の発見）を読み、各項目に安定ID＋内容ハッシュを機械付与し、
# 読める ledger.md と、裁定を貯める rulings.yaml を生成する。
#
# 追跡器の原則（judgments-are-reusable-while-subject-unchanged）:
#   裁定は「候補の文字列」でなく「対象の同一性(id)＋内容ハッシュ」に束縛する。
#   再 sweep で ledger.yaml を作り直しても:
#     - ハッシュ不変 → rulings.yaml の前回裁定をそのまま繰越（RULED）
#     - ハッシュ変化 → その行だけ再裁定に上げる（STALE=要再裁定）
#     - 新規       → PENDING（要裁定）
#   id は位置でなく内容から決まるので、項目の増減で番号がずれない。
#
# 使い方: python gen.py

import sys, os, hashlib, re
import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
LEDGER = os.path.join(HERE, "ledger.yaml")
RULINGS = os.path.join(HERE, "rulings.yaml")
OUT_MD = os.path.join(HERE, "ledger.md")

PREFIX = {
    "未カバー": "UNCOV",
    "弱い": "WEAK",
    "機能欠落": "FEAT",
    "太いVO": "FATVO",
    "過剰VO": "EXCESS",
    "抽出漏れ疑い": "EXTRACT",
}
# 表示順（class セクションの並び）
ORDER = ["未カバー", "弱い", "機能欠落", "太いVO", "過剰VO", "抽出漏れ疑い"]


def norm(s):
    return re.sub(r"\s+", " ", (s or "").strip())


def content_hash(item):
    # 裁定の対象になる実体＝義務・分類・関連VO。提案(私の助言)は同一性に含めない。
    basis = norm(item.get("obligation")) + "|" + norm(item.get("class")) + "|" + norm(item.get("related_vo"))
    return hashlib.sha256(basis.encode("utf-8")).hexdigest()


def assign_ids(items):
    seen = {}
    for it in items:
        h = content_hash(it)
        pfx = PREFIX.get(it.get("class"), "ITEM")
        short = h[:8]
        iid = f"{pfx}-{short}"
        # 万一の短縮衝突は桁を伸ばす
        while iid in seen and seen[iid] != h:
            short = h[: len(short) + 2]
            iid = f"{pfx}-{short}"
        seen[iid] = h
        it["_id"] = iid
        it["_hash"] = h
    return items


def load_rulings():
    if not os.path.exists(RULINGS):
        return {}
    with open(RULINGS, encoding="utf-8") as f:
        data = yaml.safe_load(f) or {}
    return {r["id"]: r for r in data.get("rulings", [])}


def save_rulings(items, prior):
    # 既存裁定を保存しつつ、新規idを PENDING で追加。id は残す（削除された項目の裁定履歴も消さない）。
    out = []
    current_ids = set()
    for it in items:
        iid, h = it["_id"], it["_hash"]
        current_ids.add(iid)
        p = prior.get(iid)
        if p:
            row = dict(p)
            row["id"] = iid
            row["obligation_hash"] = h  # 現ハッシュを記録（差分判定は ruled_hash と比較）
            out.append(row)
        else:
            out.append({
                "id": iid,
                "obligation_hash": h,
                "ruled_hash": "",          # 裁定時のハッシュ。裁定したら obligation_hash をここへ写す。
                "ruling": "",              # 追加 / 統合 / 分割 / §11移送 / 対象外 / 削除 / 現状維持
                "note": "",
            })
    # 現台帳から消えた過去項目の裁定も歴史として残す
    for iid, p in prior.items():
        if iid not in current_ids:
            row = dict(p); row["id"] = iid; row["_absent"] = True
            out.append(row)
    with open(RULINGS, "w", encoding="utf-8") as f:
        f.write("# VO 裁定記録。Owner が ruling を埋める。\n")
        f.write("# ruling: 追加 / 統合 / 分割 / §11移送 / 対象外 / 削除 / 現状維持 のいずれか。\n")
        f.write("# 裁定したら ruled_hash に obligation_hash の値を写す（繰越判定の基準）。\n")
        f.write("# _absent: true は現台帳から消えた過去項目（履歴保持）。\n")
        yaml.safe_dump({"rulings": out}, f, allow_unicode=True, sort_keys=False, width=1000)


def status_of(it, prior):
    p = prior.get(it["_id"])
    if not p or not p.get("ruling"):
        return "未裁定"
    if p.get("ruled_hash") and p["ruled_hash"] == it["_hash"]:
        return f"裁定済: {p['ruling']}"
    return f"要再裁定（前回: {p.get('ruling','')}）"


def esc(s):
    return norm(s).replace("|", "\\|")


def main():
    with open(LEDGER, encoding="utf-8") as f:
        data = yaml.safe_load(f)
    items = data["items"]
    assign_ids(items)
    prior = load_rulings()

    by_class = {c: [it for it in items if it.get("class") == c] for c in ORDER}
    counts = {c: len(by_class[c]) for c in ORDER}
    total = len(items)

    # 対立点（proposal に「対立」「裁定材料」を含む＝Owner 判断が本質のもの）
    conflicts = [it for it in items if ("対立" in (it.get("proposal") or "")) or ("裁定材料" in (it.get("proposal") or ""))]

    o = []
    o.append("# VO 裁定台帳")
    o.append("")
    o.append("4つの独立監査（系統的照合表 / 全体レンズ / 局所エッジ / VO太さ）の負の発見を意味ベースで重複排除し、Owner が行ごとに裁定できる形に束ねたもの。生の発見127件 → 重複排除後 **{}件**。".format(total))
    o.append("")
    o.append("**凡例（class）**:")
    o.append("- **未カバー**: 検証すべき義務はあるが、確かめる VO が無い。")
    o.append("- **弱い**: VO はあるが、その義務を十分に確かめていない。")
    o.append("- **機能欠落**: 条項が要求する機能・項目が詳細設計に無い（VO 以前の設計の穴）。")
    o.append("- **太いVO**: 既存 VO の境界が太く、説明からテストケースが一意に決まらない（薄い VO へ分割 or 判断部分を §11 へ移送）。")
    o.append("- **過剰VO**: 義務でないのに存在する、または期待状態が仕様と食い違う VO（修正・削除）。")
    o.append("- **抽出漏れ疑い**: どの VO も他監査も拾えていなかった義務。")
    o.append("")
    o.append("**id とハッシュ**: 各行の id は義務内容から決まる安定キー（位置で変わらない）。裁定は `rulings.yaml` に id で束縛され、再 sweep で義務が不変なら前回裁定を繰越、変化した行だけ「要再裁定」に上がる。")
    o.append("")
    o.append("**要約**:")
    o.append("")
    o.append("| class | 件数 |")
    o.append("|---|---|")
    for c in ORDER:
        o.append(f"| {c} | {counts[c]} |")
    o.append(f"| **合計** | **{total}** |")
    o.append("")
    o.append("> severity は各元レポートを引き継ぎ、割れた場合は高い方。照合表は行別 severity を持たないため、帰結が偽合格経路のものを高・他を中と補間している（機械的補間である旨を開示）。")
    o.append("")
    o.append("## Owner に裁定してほしいこと")
    o.append("")
    o.append("各行について、**追加 / 統合 / 分割 / §11移送 / 対象外 / 削除 / 現状維持** のいずれかを選んでほしい。私の提案は「私の提案」列にあるが、これは助言であって決定ではない。特に下の「§0 まず裁定が要る対立点」は、監査どうしで判断が割れており Owner のスコープ判断が本質的に要る箇所。")
    o.append("")

    # §0 対立点を先頭に
    o.append(f"## §0 まず裁定が要る対立点（{len(conflicts)}件）")
    o.append("")
    o.append("監査が「対象外」と「VO化して追加」に割れた、またはスコープの線引きが Owner 判断になる項目。ここだけは提案を採らず両論を残してある。")
    o.append("")
    o.append("| id | sev | 義務 | 出典 | 対立の中身（私の提案列に明記） | 裁定状態 |")
    o.append("|---|---|---|---|---|---|")
    for it in conflicts:
        o.append("| {} | {} | {} | {} | {} | {} |".format(
            it["_id"], esc(it.get("severity")), esc(it.get("obligation")), esc(it.get("source")),
            esc(it.get("proposal")), status_of(it, prior)))
    o.append("")

    # class 別
    for c in ORDER:
        rows = by_class[c]
        if not rows:
            continue
        o.append(f"## {c}（{len(rows)}件）")
        o.append("")
        o.append("| id | sev | 義務 | 出典 | 関連VO | 私の提案 | 裁定状態 |")
        o.append("|---|---|---|---|---|---|---|")
        # severity 高→中→低
        rank = {"高": 0, "中": 1, "低": 2}
        for it in sorted(rows, key=lambda x: rank.get(x.get("severity"), 3)):
            o.append("| {} | {} | {} | {} | {} | {} | {} |".format(
                it["_id"], esc(it.get("severity")), esc(it.get("obligation")), esc(it.get("source")),
                esc(it.get("related_vo")), esc(it.get("proposal")), status_of(it, prior)))
        o.append("")

    with open(OUT_MD, "w", encoding="utf-8") as f:
        f.write("\n".join(o))

    save_rulings(items, prior)

    # 集計を stderr へ
    ruled = sum(1 for it in items if status_of(it, prior).startswith("裁定済"))
    stale = sum(1 for it in items if status_of(it, prior).startswith("要再裁定"))
    pending = total - ruled - stale
    sys.stderr.write(
        f"生成: {OUT_MD}\n合計 {total} / 未裁定 {pending} / 裁定済 {ruled} / 要再裁定 {stale} / 対立点 {len(conflicts)}\n"
    )


if __name__ == "__main__":
    main()
