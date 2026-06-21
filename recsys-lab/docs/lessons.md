# Lessons — Batch 1: order beats preference, and tuning cuts both ways

What the eval actually showed. Grounded in `run.py` on MovieLens-1M, 2026-06-21.

## Setup

- **Data:** MovieLens-1M, 6040 users × 3706 items, 994k train interactions.
- **Framing:** implicit (every rating = positive); **leave-one-out** (each user's
  chronologically last interaction held out).
- **Eval:** full-ranking over all items minus train (NO sampled negatives),
  Recall@K / NDCG@K / MRR / Coverage@K. Ties = best-case rank. (Hygiene choices
  logged because they move the numbers.)

## Final leaderboard

| Model | Recall@10 | NDCG@10 | MRR | Recall@20 | NDCG@20 | Cov@10 | fit | vs Pop |
|---|---|---|---|---|---|---|---|---|
| **SASRec** (2 blocks, d=64, 200ep) | **0.2151** | **0.1088** | **0.0940** | 0.3353 | 0.1392 | 0.667 | 307s | +388% |
| Markov (1st-order) | 0.1637 | 0.0824 | 0.0736 | 0.2493 | 0.1039 | 0.733 | 0.1s | +270% |
| ItemKNN (cosine, k=100) | 0.0745 | 0.0400 | 0.0384 | 0.1114 | 0.0493 | 0.138 | 1.7s | +79% |
| BPR (f=64, epochs=120) | 0.0735 | 0.0360 | 0.0362 | 0.1263 | 0.0493 | 0.506 | 158s | +61% |
| ALS (f=64, iters=15, α=40) | 0.0710 | 0.0341 | 0.0352 | 0.1315 | 0.0493 | 0.628 | 354s | +53% |
| Popularity | 0.0437 | 0.0223 | 0.0231 | 0.0719 | 0.0294 | 0.053 | 0.2s | baseline |

## What we learned

1. **Order is the dominant signal on ML-1M.** Both sequence models (SASRec,
   Markov) sit far above all three order-blind models (ItemKNN/ALS/BPR). Knowing
   *what you watched last* beats knowing *everything you watched, unordered*. The
   MF/KNN models discard the signal that matters most here. (Caveat: this is a
   property of ML-1M + next-item LOO; an open test on our own data.)

2. **The transformer earns its keep — over the dumb sequence model.** SASRec beat
   Markov +31% on Recall@10 (0.2151 vs 0.1637), so there's real higher-order
   sequential structure beyond the last item, and self-attention captures it.
   (Prediction logged beforehand was "ties/barely wins" — wrong; recorded so the
   miss is on the books.)

3. **★ Tuning rigor cuts BOTH ways — the session's sharpest lesson.** Dacrema's
   critique: fancy models *falsely look good* when baselines are under-tuned. We
   hit the **mirror image** twice:
   - BPR at 25 epochs looked worst of the MF models → 120 epochs, +28% NDCG,
     jumped past ALS.
   - **SASRec's first run came dead last** (Recall@10 0.0576) — under-trained:
     loss still falling at epoch 100, plus PyTorch's default `nn.Embedding`
     std≈1 made d=64 logits std≈8, burning ~20 epochs just shrinking them. Fixed
     init (std 0.01) + 200 epochs → loss 1.15→0.42 → **3.7× better, first place.**

   A fair comparison requires *every* model — dumb and fancy — to be properly
   trained. The eval-first reflex (never conclude from an unconverged loss) saved
   us from the exact-wrong headline in *both* directions in one batch.

4. **Value ≠ accuracy.** Markov gets 76% of SASRec's Recall@10 in **0.1s with ~10
   lines of counting**, vs SASRec's 307s of MPS training. Ship Markov (or a
   hybrid) for value; reach for SASRec for max accuracy.

5. **Among order-blind models, simplest wins top-10** (Dacrema in miniature):
   ItemKNN ≈ tuned BPR > ALS at K=10; ALS/BPR catch up by K=20 with far higher
   coverage (0.5–0.63 vs ItemKNN's 0.14).

6. **The "low" numbers are the honest ones.** Recall@10 0.04–0.22 because we rank
   against all 3706 items; the common 100-sampled-negatives protocol inflates
   these ~10×. Protocol dominates the headline — we took the honest road.

7. **Personalization works here** — everything beats popularity (+53% to +388%),
   the opposite of event-forecast. This data has signal and the harness rewards it.

## Next

- **Rust speed port** — accuracy picture is locked. ALS (354s) and SASRec (307s)
  are the painful ones; Markov/ItemKNN are already instant.
- **Run the ladder on our own data** (Olist / anime_list): does order dominate
  there too, or is it an event-forecast-style flatline?
- Optional: hyperparameter sweep ALS/ItemKNN for a fully fair order-blind tier;
  try a Markov+MF hybrid (cheap, might close much of the SASRec gap).
