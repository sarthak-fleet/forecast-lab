# PRD: Alerts and Decision Rules

## Summary

Turn forecasts into actionable alerts by letting users define thresholds on
hot zones, cooling zones, uncertainty, and anomaly behavior.

## Problem

The system can already tell us what is likely next, but it still depends on a
human to notice when the forecast crosses a meaningful threshold. That is fine
for a demo, but it leaves the product one step short of operational value.

Users need a way to ask: "When should I care?"

## Target User

Operators who want a concise alert stream for changing demand, capacity
pressure, or unusual event timing.

## Desired Behavior

- Let users define alert conditions for:
  - hot zone growth
  - cooling zone collapse
  - forecast confidence drops
  - anomaly spikes
  - mix shifts by service or product type
- Evaluate alerts against both observed and predicted windows.
- Produce a short reason for each alert.
- Support a low-noise default profile suitable for the existing decision
  surface.

## Non-Goals

- No paging, email, or chat delivery in the first slice.
- No complex rule editor UI.
- No auto-remediation or workflow execution.

## Proposed Scope

1. Add a rule model that can evaluate:
   - threshold comparisons
   - percentage deltas
   - confidence floors
   - anomaly counts over a window
2. Generate alerts from the same payloads that feed `/decision-report` and
   `/action-report`.
3. Include alert summaries in the explorer and report outputs.
4. Add tests that verify:
   - noisy streams do not over-alert
   - genuine hot-zone surges trigger exactly once per window
   - low-confidence forecasts are surfaced distinctly from anomalies

## User Stories

- As an operator, I want a short list of actionable alerts instead of having to
  scan the full decision report.
- As a team lead, I want low-noise defaults so the system only interrupts when
  something material changes.
- As a reviewer, I want each alert to explain why it fired and what part of the
  forecast triggered it.

## API And UI Notes

- Make the first implementation in-process and stateless.
- Support both observed and predicted values in the same rule evaluation pass.
- Attach a short reason, severity, and triggering window to every alert.
- Keep delivery integrations out of scope until the rule shape is stable.

## Success Proof

- The system emits clear alerts when a forecasted hot zone crosses a defined
  threshold.
- The alert output is explainable and stable across repeated runs.
- The default configuration stays quiet unless the stream really changes.

## Delivery Checklist

- Add a quiet default profile before exposing custom thresholds.
- Confirm the decision-report narrative can be reused as alert context.
- Add regression fixtures for both surging and quiet streams.

## Rollout Notes

Start with in-process rule evaluation. Persisted alert subscriptions and
delivery integrations can come later.
