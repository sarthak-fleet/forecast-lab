use crate::Event;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocationCoords {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BucketLocationMeta {
    pub bucket: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lng: Option<f64>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct LocationResolver {
    catalog: HashMap<String, LocationCoords>,
}

impl LocationResolver {
    pub fn new(catalog: HashMap<String, LocationCoords>) -> Self {
        Self { catalog }
    }

    pub fn default_demo() -> Self {
        Self::new(demo_catalog())
    }

    pub fn catalog(&self) -> &HashMap<String, LocationCoords> {
        &self.catalog
    }

    pub fn resolve_bucket(&self, bucket: &str, event: Option<&Event>, bucket_field: &str) -> BucketLocationMeta {
        if let Some(event) = event {
            if let Some(resolved) = self.resolve_from_event(event, bucket_field) {
                if resolved.bucket == bucket {
                    return resolved;
                }
            }
        }

        let key = normalize_key(bucket);
        if let Some(coords) = self.catalog.get(&key) {
            return BucketLocationMeta {
                bucket: bucket.to_string(),
                label: bucket.to_string(),
                lat: Some(coords.lat),
                lng: Some(coords.lng),
                source: "catalog".to_string(),
            };
        }

        BucketLocationMeta {
            bucket: bucket.to_string(),
            label: bucket.to_string(),
            lat: None,
            lng: None,
            source: "fallback".to_string(),
        }
    }

    pub fn resolve_from_event(&self, event: &Event, bucket_field: &str) -> Option<BucketLocationMeta> {
        let label = property_string(event, bucket_field).unwrap_or_else(|| "unknown".to_string());

        if let Some((lat, lng)) = extract_coordinates(&event.properties) {
            let bucket = property_string(event, bucket_field)
                .unwrap_or_else(|| format!("{lat:.4},{lng:.4}"));
            return Some(BucketLocationMeta {
                bucket,
                label: label.clone(),
                lat: Some(lat),
                lng: Some(lng),
                source: "coordinates".to_string(),
            });
        }

        let key = normalize_key(&label);
        if let Some(coords) = self.catalog.get(&key) {
            return Some(BucketLocationMeta {
                bucket: label.clone(),
                label,
                lat: Some(coords.lat),
                lng: Some(coords.lng),
                source: "catalog".to_string(),
            });
        }

        if label != "unknown" {
            return Some(BucketLocationMeta {
                bucket: label.clone(),
                label,
                lat: None,
                lng: None,
                source: "fallback".to_string(),
            });
        }

        None
    }

    pub fn collect_bucket_locations(
        &self,
        events: &[Event],
        bucket_values: &[String],
        bucket_field: &str,
    ) -> BTreeMap<String, BucketLocationMeta> {
        let mut locations = BTreeMap::new();
        for bucket in bucket_values {
            if locations.contains_key(bucket) {
                continue;
            }
            let sample = events
                .iter()
                .find(|event| property_string(event, bucket_field).as_deref() == Some(bucket.as_str()));
            locations.insert(
                bucket.clone(),
                self.resolve_bucket(bucket, sample, bucket_field),
            );
        }
        locations
    }
}

pub fn demo_catalog() -> HashMap<String, LocationCoords> {
    HashMap::from([
        (
            "koramangala".to_string(),
            LocationCoords {
                lat: 12.9352,
                lng: 77.6245,
            },
        ),
        (
            "indiranagar".to_string(),
            LocationCoords {
                lat: 12.9719,
                lng: 77.6412,
            },
        ),
        (
            "whitefield".to_string(),
            LocationCoords {
                lat: 12.9698,
                lng: 77.7500,
            },
        ),
        (
            "hsr".to_string(),
            LocationCoords {
                lat: 12.9116,
                lng: 77.6473,
            },
        ),
    ])
}

pub fn parse_location_catalog(
    raw: Option<HashMap<String, LocationCoords>>,
) -> LocationResolver {
    match raw {
        Some(catalog) if !catalog.is_empty() => LocationResolver::new(
            catalog
                .into_iter()
                .map(|(key, coords)| (normalize_key(&key), coords))
                .collect(),
        ),
        _ => LocationResolver::default_demo(),
    }
}

fn normalize_key(value: &str) -> String {
    value.trim().to_lowercase()
}

fn property_string(event: &Event, field: &str) -> Option<String> {
    match event.properties.get(field)? {
        Value::String(value) if !value.trim().is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn extract_coordinates(properties: &Map<String, Value>) -> Option<(f64, f64)> {
    let lat = coordinate_value(properties, &["lat", "latitude"])?;
    let lng = coordinate_value(properties, &["lng", "lon", "longitude"])?;
    Some((lat, lng))
}

fn coordinate_value(properties: &Map<String, Value>, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(value) = properties.get(*key).and_then(numeric_value) {
            return Some(value);
        }
    }
    None
}

fn numeric_value(raw: &Value) -> Option<f64> {
    match raw {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use pretty_assertions::assert_eq;
    use serde_json::Map;

    fn event_with(properties: Map<String, Value>) -> Event {
        Event {
            id: None,
            ts: Utc::now(),
            event_type: "test".to_string(),
            entity_id: None,
            stream_id: None,
            properties,
        }
    }

    #[test]
    fn resolves_coordinate_backed_buckets() {
        let resolver = LocationResolver::new(HashMap::new());
        let event = event_with(Map::from_iter([
            ("location".to_string(), Value::String("zone-a".to_string())),
            ("lat".to_string(), Value::Number(serde_json::Number::from_f64(40.7128).unwrap())),
            (
                "lng".to_string(),
                Value::Number(serde_json::Number::from_f64(-74.0060).unwrap()),
            ),
        ]));
        let resolved = resolver.resolve_from_event(&event, "location").unwrap();
        assert_eq!(resolved.bucket, "zone-a");
        assert_eq!(resolved.source, "coordinates");
        assert_eq!(resolved.lat, Some(40.7128));
        assert_eq!(resolved.lng, Some(-74.0060));
    }

    #[test]
    fn resolves_name_backed_buckets_from_catalog() {
        let resolver = LocationResolver::new(HashMap::from([(
            "downtown".to_string(),
            LocationCoords {
                lat: 37.7749,
                lng: -122.4194,
            },
        )]));
        let event = event_with(Map::from_iter([(
            "location".to_string(),
            Value::String("Downtown".to_string()),
        )]));
        let resolved = resolver.resolve_from_event(&event, "location").unwrap();
        assert_eq!(resolved.bucket, "Downtown");
        assert_eq!(resolved.source, "catalog");
        assert_eq!(resolved.lat, Some(37.7749));
    }

    #[test]
    fn resolves_fallback_buckets_without_coordinates() {
        let resolver = LocationResolver::new(HashMap::new());
        let event = event_with(Map::from_iter([(
            "location".to_string(),
            Value::String("mystery-zone".to_string()),
        )]));
        let resolved = resolver.resolve_from_event(&event, "location").unwrap();
        assert_eq!(resolved.bucket, "mystery-zone");
        assert_eq!(resolved.source, "fallback");
        assert_eq!(resolved.lat, None);
        assert_eq!(resolved.lng, None);
    }

    #[test]
    fn demo_catalog_keeps_sample_locations_renderable() {
        let resolver = LocationResolver::default_demo();
        let event = event_with(Map::from_iter([(
            "location".to_string(),
            Value::String("koramangala".to_string()),
        )]));
        let resolved = resolver.resolve_from_event(&event, "location").unwrap();
        assert_eq!(resolved.source, "catalog");
        assert!(resolved.lat.is_some() && resolved.lng.is_some());
    }
}
