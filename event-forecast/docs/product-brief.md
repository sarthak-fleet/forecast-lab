# Product Brief

## Target User

Operators or builders who have streams of events and need to know what is likely
to happen next: where demand moves, which service type comes next, which product
category is likely to be involved, or where a user journey will flow.

## Current Failure

Raw event streams are easy to store but hard to reason about. Most analytics
tools show what already happened. The useful question is often:

- What event is likely next?
- Where will it happen?
- Which service/product type is likely involved?
- What should I prepare for?

## Desired Behavior

Given a historical stream, the system should predict a future stream with:

- expected event type
- expected timestamp
- likely location
- likely service type
- likely product type
- confidence and explanation for every prediction

## First Slice

Use a Rust + Rocket service with TimescaleDB storage and transparent baselines:

- transition counts for event type
- median inter-arrival time for expected timestamp
- previous-value to next-value transitions for properties
- event-transition-conditioned property distributions

This is not meant to be final. It is meant to make the data contract and failure
modes obvious.

## Non-Goals

- Do not build a dashboard yet.
- Do not ingest PII or private user data.
- Do not add an ML framework until baseline quality is measurable.

## Success Proof

- A JSON sample stream produces a plausible future stream.
- Tests prove event and property predictions follow learned transitions.
- The output includes confidence and explanation, not just labels.
- The code can be replaced by a stronger model later without changing the event
  contract.
