# PRD: Stream Replay and Playback

## Summary

Add a replay surface that lets users move through a historical stream over time
and compare observed movement against the forecasted future at each step.

## Problem

The product can already predict and aggregate, but it is still mostly a static
query surface. Operators need to see how demand or activity evolves window by
window in order to trust the forecast and notice regime changes.

Without replay, the user has to mentally reconstruct the timeline from a single
snapshot.

## Target User

Operators and analysts who need to inspect how a stream changed, when the shift
started, and whether the forecast tracked the movement correctly.

## Desired Behavior

- Replay the stream in fixed time windows.
- Show observed heat, predicted heat, and deltas per window.
- Allow pause, scrub, step-forward, and step-back controls.
- Keep the forecast context visible while the stream advances.
- Highlight when the forecast diverges from observed activity.

## Non-Goals

- No full video-style animation polish in the first slice.
- No collaborative cursors or shared playback state.
- No historical backfill jobs or background rendering pipeline.

## Proposed Scope

1. Add a replay API that accepts:
   - events
   - window size
   - step size
   - horizon
   - optional bucket field
2. Recompute forecast, heatmap, and decision summaries per window.
3. Expose a simple playback model to the explorer UI:
   - current window
   - play/pause
   - step controls
   - scrubber
4. Show drift markers when:
   - hot zones move
   - mix shifts materially
   - prediction confidence drops
5. Add fixtures that verify replay windows produce deterministic output.

## User Stories

- As an operator, I can scrub through the stream and see how the forecast
  changes window by window.
- As an analyst, I can compare observed movement against the forecast at a
  specific point in time.
- As a reviewer, I can rerun the same replay input and get the same output for
  inspection or regression testing.

## API And UI Notes

- Treat replay as a derived view over the same event payloads, not a new storage
  model.
- Keep playback state explicit in the response so the UI can pause, scrub, and
  step without guesswork.
- Reuse the existing heatmap and decision-report vocabulary where possible.
- Include enough metadata for the UI to render a window label, current horizon,
  and drift status per step.

## Success Proof

- A user can move through a stream and see the heatmap change over time.
- The replay surface shows where the forecast was right or wrong as the stream
  unfolds.
- The same payload can be replayed repeatedly with stable results.

## Delivery Checklist

- Fix deterministic window ordering before adding UI controls.
- Add fixture snapshots for at least one simple stream and one mixed stream.
- Keep replay additive so the existing decision surface stays unchanged.

## Rollout Notes

This should ship as an additive API and UI path. It should not change the
existing `/action-report` contract.
