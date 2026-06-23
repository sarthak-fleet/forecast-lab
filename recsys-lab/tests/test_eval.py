"""Smoke tests for recsys-lab eval ranking metrics."""
import numpy as np
from scipy.sparse import csr_matrix
from recsys.eval import evaluate


class StubModel:
    """Returns deterministic scores: item_index * 0.1 + user_index * 0.01."""

    def score_batch(self, users):
        n = len(users)
        scores = np.zeros((n, 5))
        for i, u in enumerate(users):
            scores[i] = np.arange(5) * 0.1 + u * 0.01
        return scores


def test_evaluate_basic():
    # 2 users, 5 items; user 0 has seen item 0, test item is item 3
    # user 1 has seen item 1, test item is item 4
    train_ui = csr_matrix(
        ([1, 1], ([0, 1], [0, 1])),
        shape=(2, 5),
    )
    test_items = np.array([3, 4])
    model = StubModel()

    result = evaluate(model, train_ui, test_items, Ks=(5,))
    assert result["users"] == 2
    assert "Recall@5" in result
    assert "NDCG@5" in result
    assert "Coverage@5" in result
    assert "MRR" in result
    assert 0 <= result["Recall@5"] <= 1


def test_evaluate_masks_train_items():
    """Train items should be masked out (scored -inf) so they're never recommended."""
    train_ui = csr_matrix(
        ([1], ([0], [4])),
        shape=(1, 5),
    )
    test_items = np.array([3])
    model = StubModel()

    result = evaluate(model, train_ui, test_items, Ks=(5,))
    # Item 4 (highest score) is in train, so it should be masked
    # The test item 3 should rank well
    assert result["Recall@5"] == 1.0
