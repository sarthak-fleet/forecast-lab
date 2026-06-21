"""Quick BPR tune — batch 1 used 25 epochs (undertuned). Sweep a small grid."""
from recsys.data import load_split
from recsys.eval import evaluate
from recsys.models import BPR

train_ui, test_items, n_users, n_items, _ = load_split()
print(f"users={n_users} items={n_items}\n")

grid = [
    dict(epochs=50, lr=0.05, reg=0.01, factors=64),
    dict(epochs=120, lr=0.05, reg=0.01, factors=64),
    dict(epochs=120, lr=0.1, reg=0.01, factors=64),
]
for g in grid:
    m = BPR(**g)
    m.fit(train_ui)
    r = evaluate(m, train_ui, test_items)
    print(f"{g}  ->  Recall@10={r['Recall@10']:.4f}  NDCG@10={r['NDCG@10']:.4f}  MRR={r['MRR']:.4f}")
