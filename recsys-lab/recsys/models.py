"""Reference recommender implementations — pure NumPy/scipy, readable over fast.

Common interface:
  .fit(train_ui, train_seqs=None)   train_ui: scipy CSR (users x items), binary
                                    train_seqs: list of per-user item arrays (ts order)
  .score_batch(user_ids)            -> dense ndarray (len(user_ids) x n_items)

These are the accuracy-reference implementations; the Rust port (speed pass)
will mirror them 1:1. (SASRec lives in sasrec.py — it needs torch.)
"""
import numpy as np
from scipy.sparse import diags, csr_matrix


class Popularity:
    """Non-personalized: score = global interaction count. The bar to beat."""
    def fit(self, train_ui, train_seqs=None):
        self.pop = np.asarray(train_ui.sum(axis=0)).ravel()

    def score_batch(self, users):
        return np.tile(self.pop, (len(users), 1))


class ItemKNN:
    """Item-item cosine CF with top-k neighbor truncation.

    score(u, i) = sum_{j in train(u)} cosine_sim(i, j), neighbors truncated to
    top-`knn` per item. The truncation setting is logged because it alone can
    swing metrics double digits (see roadmap eval-hygiene note).
    """
    def __init__(self, knn=100):
        self.knn = knn

    def fit(self, train_ui, train_seqs=None):
        self.train_ui = train_ui.tocsr()
        X = train_ui.T.tocsr().astype(np.float64)            # items x users
        norms = np.sqrt(np.asarray(X.multiply(X).sum(axis=1)).ravel())
        norms[norms == 0] = 1.0
        Xn = diags(1.0 / norms) @ X
        sim = np.asarray((Xn @ Xn.T).todense())              # items x items cosine
        np.fill_diagonal(sim, 0.0)
        if self.knn and self.knn < sim.shape[1]:
            for i in range(sim.shape[0]):                    # keep top-knn per row
                row = sim[i]
                drop = np.argpartition(row, -self.knn)[:-self.knn]
                row[drop] = 0.0
        self.sim = sim

    def score_batch(self, users):
        return np.asarray(self.train_ui[users] @ self.sim)


class Markov:
    """First-order item->item transition counts — the dumb sequence baseline,
    direct sibling of the event-forecast model. score(u, j) ∝ count(last(u) -> j),
    falling back to popularity when the last item has no observed successor.
    """
    def fit(self, train_ui, train_seqs=None):
        n_items = train_ui.shape[1]
        rows, cols = [], []
        for s in train_seqs:
            if len(s) >= 2:
                rows.append(s[:-1])
                cols.append(s[1:])
        rows = np.concatenate(rows) if rows else np.array([], dtype=np.int64)
        cols = np.concatenate(cols) if cols else np.array([], dtype=np.int64)
        self.T = csr_matrix((np.ones(len(rows)), (rows, cols)), shape=(n_items, n_items))
        self.last_item = np.array([s[-1] if len(s) else 0 for s in train_seqs])
        self.pop = np.asarray(train_ui.sum(axis=0)).ravel()

    def score_batch(self, users):
        S = np.asarray(self.T[self.last_item[users]].todense())
        cold = S.sum(axis=1) == 0                            # last item had no successor
        if cold.any():
            S[cold] = self.pop
        return S


class ALS:
    """Implicit-feedback matrix factorization (Hu, Koren, Volinsky 2008).

    Confidence c_ui = 1 + alpha on observed pairs; alternating least squares.
    """
    def __init__(self, factors=64, iters=15, reg=0.1, alpha=40.0, seed=0):
        self.f, self.iters, self.reg, self.alpha, self.seed = factors, iters, reg, alpha, seed

    def fit(self, train_ui, train_seqs=None):
        R = train_ui.tocsr()
        Rt = train_ui.T.tocsr()
        n_users, n_items = R.shape
        rng = np.random.default_rng(self.seed)
        self.U = 0.01 * rng.standard_normal((n_users, self.f))
        self.V = 0.01 * rng.standard_normal((n_items, self.f))
        for _ in range(self.iters):
            self._solve(self.U, self.V, R)
            self._solve(self.V, self.U, Rt)

    def _solve(self, X, Y, M):
        f = self.f
        YtY = Y.T @ Y
        lreg = self.reg * np.eye(f)
        indptr, indices = M.indptr, M.indices
        for e in range(X.shape[0]):
            idx = indices[indptr[e]:indptr[e + 1]]
            if len(idx) == 0:
                X[e] = 0.0
                continue
            Yi = Y[idx]
            A = YtY + self.alpha * (Yi.T @ Yi) + lreg
            b = (1.0 + self.alpha) * Yi.sum(axis=0)
            X[e] = np.linalg.solve(A, b)

    def score_batch(self, users):
        return self.U[users] @ self.V.T


class BPR:
    """Bayesian Personalized Ranking MF (Rendle 2009) via vectorized SGD."""
    def __init__(self, factors=64, epochs=25, lr=0.05, reg=0.01, batch=8192, seed=0):
        self.f, self.epochs, self.lr, self.reg, self.batch, self.seed = \
            factors, epochs, lr, reg, batch, seed

    def fit(self, train_ui, train_seqs=None):
        coo = train_ui.tocoo()
        pu, pi = coo.row, coo.col
        n_pairs = len(pu)
        n_users, n_items = train_ui.shape
        rng = np.random.default_rng(self.seed)
        U = 0.01 * rng.standard_normal((n_users, self.f))
        V = 0.01 * rng.standard_normal((n_items, self.f))
        steps = max(1, n_pairs // self.batch)
        for _ in range(self.epochs):
            for _ in range(steps):
                p = rng.integers(0, n_pairs, self.batch)
                u, i = pu[p], pi[p]
                j = rng.integers(0, n_items, self.batch)
                xu, vi, vj = U[u], V[i], V[j]
                x = np.sum(xu * (vi - vj), axis=1)
                sig = 1.0 / (1.0 + np.exp(x))               # sigmoid(-x): gradient coeff
                s = sig[:, None]
                np.add.at(U, u, self.lr * (s * (vi - vj) - self.reg * xu))
                np.add.at(V, i, self.lr * (s * xu - self.reg * vi))
                np.add.at(V, j, self.lr * (-s * xu - self.reg * vj))
        self.U, self.V = U, V

    def score_batch(self, users):
        return self.U[users] @ self.V.T
