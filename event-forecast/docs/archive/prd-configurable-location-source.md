# PRD: Configurable Location Source

## Summary

Replace the built-in Bangalore-only coordinate table with a configurable
location source so the heatmap and explorer UI work on arbitrary event streams
without code changes.

## Problem

The current location rendering path is tied to a sample-specific coordinate
table. That makes the map useful for the bundled data, but brittle for any real
stream that uses different neighborhoods, cities, campuses, routes, or bucket
keys.

The product promise is a general stream-to-decision engine. A fixed location
table breaks that promise as soon as the first non-sample stream lands.

## Target User

Operators, analysts, or builders who ingest their own stream and want a map
surface that reflects their geography instead of the demo dataset.

## Desired Behavior

- Accept lat/lng directly on events when present.
- Resolve location buckets through a configurable source instead of a hardcoded
  table.
- Support a fallback lookup for named locations when coordinates are absent.
- Keep observed and predicted heatmap rows renderable even when a stream has no
  predefined geography.
- Preserve the current explorer and heatmap API contracts.

## Non-Goals

- No geocoding service integration in the first slice.
- No route-level optimization or map matching.
- No multi-tenant location catalog management UI.

## Proposed Scope

1. Add a location-resolution layer that can derive a stable bucket from:
   - raw lat/lng
   - an injected location catalog
   - a deterministic fallback key when geometry is missing
2. Extend heatmap generation to carry the resolved location metadata alongside
   the bucket key.
3. Update the explorer to render arbitrary streams without depending on the
   sample coordinate list.
4. Add fixture-backed tests for:
   - coordinate-backed buckets
   - name-backed buckets
   - fallback buckets when no coordinates exist

## User Stories

- As an operator, I can send a stream with lat/lng fields and see the map render
  without editing code.
- As an analyst, I can provide named locations from my own catalog and get
  stable aggregation buckets.
- As a builder, I can ingest sparse data and still get a usable explorer surface
  instead of a broken map.

## API And UI Notes

- Keep `/heatmap` and `/action-report` shape-compatible.
- Accept location data from either top-level fields or nested `properties`.
- Preserve a readable label for each bucket so the explorer can show a human
  name even when the bucket key is derived.
- Prefer deterministic derivation over heuristic geocoding in the first slice.

## Success Proof

- A new stream can render in `/explorer` without editing source code.
- `/heatmap` still returns usable rows when a location is only a string label.
- Tests prove the same event stream can be rendered with a different location
  source without changing the API shape.

## Delivery Checklist

- Add resolver tests before wiring UI changes.
- Update the sample explorer data path only after the resolver is stable.
- Verify the demo dataset still renders as before.

## Rollout Notes

Ship as a backward-compatible change. The demo data should continue to render,
but the rendering path should no longer depend on it.
