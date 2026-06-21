"""Train models and print + save a leaderboard.

Usage:
  python3 run.py                 # run all models
  python3 run.py markov bpr      # run only matching models, merge into cached results.json
                                 # (lets us add a model without re-paying ALS's runtime)
"""
import json
import sys
import time
from pathlib import Path

from recsys.data import load_split
from recsys.eval import evaluate
from recsys.models import Popularity, ItemKNN, ALS, BPR, Markov

HERE = Path(__file__).resolve().parent


def _sasrec():
    from recsys.sasrec import SASRec
    return SASRec(epochs=200)


MODELS = [
    ("Popularity", lambda: Popularity()),
    ("ItemKNN (cosine, k=100)", lambda: ItemKNN(knn=100)),
    ("Markov (1st-order)", lambda: Markov()),
    ("ALS (f=64, iters=15, a=40)", lambda: ALS()),
    ("BPR (f=64, epochs=120)", lambda: BPR(epochs=120)),
    ("SASRec (2 blocks, d=64)", _sasrec),
]


def leaderboard(rows, n_items):
    rows = sorted(rows, key=lambda r: -r["NDCG@10"])
    pop = next((r for r in rows if r["model"] == "Popularity"), None)
    cols = ["Recall@10", "NDCG@10", "MRR", "Recall@20", "NDCG@20", "Coverage@10", "fit_s"]
    head = (f"users={pop['users'] if pop else '?'} · items={n_items} · protocol: "
            "leave-one-out, full-ranking (no sampled negatives), ties = best-case rank.")
    md = ["# Leaderboard — MovieLens-1M (LOO, full-ranking)", "", head, "",
          "| Model | " + " | ".join(cols) + " | vs Pop NDCG@10 |",
          "|" + "---|" * (len(cols) + 2)]
    for r in rows:
        if pop and pop["NDCG@10"]:
            tag = "baseline" if r["model"] == "Popularity" else f"{(r['NDCG@10']/pop['NDCG@10']-1)*100:+.0f}%"
        else:
            tag = "-"
        md.append("| " + r["model"] + " | " + " | ".join(str(r[c]) for c in cols) + f" | {tag} |")
    return "\n".join(md) + "\n", rows


def main():
    sel = [a.lower() for a in sys.argv[1:]]
    train_ui, test_items, n_users, n_items, train_seqs = load_split()
    print(f"MovieLens-1M users={n_users} items={n_items} train={train_ui.nnz} (LOO)\n")

    merged = {}
    res_json = HERE / "results.json"
    if sel and res_json.exists():
        for r in json.loads(res_json.read_text()):
            merged[r["model"]] = r

    for name, factory in MODELS:
        if sel and not any(s in name.lower() for s in sel):
            continue
        m = factory()
        t = time.time()
        m.fit(train_ui, train_seqs)
        res = evaluate(m, train_ui, test_items)
        res["model"] = name
        res["fit_s"] = round(time.time() - t, 1)
        merged[name] = res
        print(f"{name:30s} Recall@10={res['Recall@10']:.4f} NDCG@10={res['NDCG@10']:.4f} "
              f"MRR={res['MRR']:.4f} Cov@10={res['Coverage@10']:.3f} ({res['fit_s']}s)")

    md, rows = leaderboard(list(merged.values()), n_items)
    res_json.write_text(json.dumps(rows, indent=2))
    (HERE / "results.md").write_text(md)
    print("\n" + md)


if __name__ == "__main__":
    main()
