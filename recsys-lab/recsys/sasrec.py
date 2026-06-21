"""SASRec — Self-Attentive Sequential Recommendation (Kang & McAuley 2018).

A causal Transformer over a user's interaction sequence; the last position's
hidden state scores the next item. Trained with the original SASRec objective:
per-position binary ranking of the true next item vs. one sampled negative.

Item ids are shifted +1 so 0 can be the padding index; sequences are left-padded
to `maxlen` (most recent item last). Same .fit/.score_batch interface as the
NumPy models so it drops into the shared eval harness.
"""
import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F


def _left_pad(seqs, maxlen):
    out = np.zeros((len(seqs), maxlen), dtype=np.int64)
    for r, s in enumerate(seqs):
        s = s[-maxlen:]
        out[r, maxlen - len(s):] = s
    return out


class _Net(nn.Module):
    def __init__(self, n_items, maxlen, d, n_blocks, n_heads, dropout):
        super().__init__()
        self.item_emb = nn.Embedding(n_items + 1, d, padding_idx=0)
        self.pos_emb = nn.Embedding(maxlen, d)
        self.drop = nn.Dropout(dropout)
        self.blocks = nn.ModuleList([nn.ModuleDict({
            "ln1": nn.LayerNorm(d),
            "attn": nn.MultiheadAttention(d, n_heads, dropout=dropout, batch_first=True),
            "ln2": nn.LayerNorm(d),
            "ff": nn.Sequential(nn.Linear(d, d), nn.ReLU(), nn.Dropout(dropout), nn.Linear(d, d)),
        }) for _ in range(n_blocks)])
        self.last_ln = nn.LayerNorm(d)
        # small init: default nn.Embedding std≈1 makes d=64 dot-product logits
        # std≈8, wasting early epochs shrinking them. Standard SASRec inits small.
        nn.init.normal_(self.item_emb.weight, std=0.01)
        nn.init.normal_(self.pos_emb.weight, std=0.01)
        with torch.no_grad():
            self.item_emb.weight[0].zero_()                  # keep padding row at 0

    def forward(self, seq):                                   # seq: (B, L) ids, 0=pad
        B, L = seq.shape
        pos = torch.arange(L, device=seq.device).unsqueeze(0).expand(B, L)
        x = self.drop(self.item_emb(seq) + self.pos_emb(pos))
        pad = seq == 0
        causal = torch.triu(torch.ones(L, L, device=seq.device, dtype=torch.bool), 1)
        for b in self.blocks:
            h = b["ln1"](x)
            a, _ = b["attn"](h, h, h, attn_mask=causal, key_padding_mask=pad, need_weights=False)
            a = torch.nan_to_num(a)                           # isolate all-masked pad rows
            x = x + a
            x = x + b["ff"](b["ln2"](x))
        return self.last_ln(x)                                # (B, L, d)


class SASRec:
    def __init__(self, maxlen=200, d=64, n_blocks=2, n_heads=1, dropout=0.2,
                 epochs=80, lr=1e-3, batch=128, seed=0):
        self.maxlen, self.d, self.n_blocks, self.n_heads, self.dropout = \
            maxlen, d, n_blocks, n_heads, dropout
        self.epochs, self.lr, self.batch, self.seed = epochs, lr, batch, seed

    def fit(self, train_ui, train_seqs=None):
        torch.manual_seed(self.seed)
        self.dev = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
        self.n_items = train_ui.shape[1]
        self.seqs = [np.asarray(s) + 1 for s in train_seqs]   # shift +1 (0 = pad)
        self.net = _Net(self.n_items, self.maxlen, self.d,
                        self.n_blocks, self.n_heads, self.dropout).to(self.dev)
        opt = torch.optim.Adam(self.net.parameters(), lr=self.lr, betas=(0.9, 0.98))
        rng = np.random.default_rng(self.seed)
        users = np.array([u for u, s in enumerate(self.seqs) if len(s) >= 2])

        self.net.train()
        for ep in range(self.epochs):
            rng.shuffle(users)
            total = 0.0
            for st in range(0, len(users), self.batch):
                bu = users[st:st + self.batch]
                inp = _left_pad([self.seqs[u][:-1] for u in bu], self.maxlen)
                lab = _left_pad([self.seqs[u][1:] for u in bu], self.maxlen)
                neg = rng.integers(1, self.n_items + 1, size=lab.shape)
                neg = np.where(lab == 0, 0, neg)
                inp_t = torch.from_numpy(inp).to(self.dev)
                lab_t = torch.from_numpy(lab).to(self.dev)
                neg_t = torch.from_numpy(neg).to(self.dev)

                h = self.net(inp_t)                           # (B, L, d)
                pos_e = self.net.item_emb(lab_t)
                neg_e = self.net.item_emb(neg_t)
                pos_l = (h * pos_e).sum(-1)
                neg_l = (h * neg_e).sum(-1)
                mask = (lab_t != 0).float()
                loss = -((F.logsigmoid(pos_l) + F.logsigmoid(-neg_l)) * mask).sum() / mask.sum()

                opt.zero_grad()
                loss.backward()
                opt.step()
                total += loss.item()
            if ep == 0 or (ep + 1) % 20 == 0:
                print(f"    SASRec epoch {ep + 1}/{self.epochs} loss={total / max(1, len(users) // self.batch):.4f}")

    @torch.no_grad()
    def score_batch(self, users):
        self.net.eval()
        out = np.empty((len(users), self.n_items), dtype=np.float32)
        for st in range(0, len(users), 256):
            bu = users[st:st + 256]
            seq = _left_pad([self.seqs[u] for u in bu], self.maxlen)
            h = self.net(torch.from_numpy(seq).to(self.dev))[:, -1, :]   # (b, d)
            logits = h @ self.net.item_emb.weight.T                       # (b, n_items+1)
            out[st:st + len(bu)] = logits[:, 1:].cpu().numpy()            # drop pad col
        return out
