//! `MatchingData` — Type-erased data that flows between `DataInput` and `InputMatcher`
//!
//! This is the key insight from Envoy's design: type erasure at the data level.
//! `DataInputs` produce `MatchingData`, and `InputMatchers` consume it.
//! This allows `InputMatchers` to be non-generic and shareable across contexts.
//!
//! # Extensibility via `Custom`
//!
//! For domain-specific types not covered by the primitives, implement
//! [`CustomMatchData`] and wrap in `MatchingData::Custom(Arc::new(your_type))`.

use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

/// Extension trait for custom matching data types.
///
/// This enables users to extend `MatchingData` without modifying the core.
/// Implement this trait for domain-specific types, then wrap with
/// `MatchingData::Custom(Arc::new(your_type))`.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to support concurrent evaluation
/// and FFI use cases.
///
/// # Example
///
/// ```
/// use std::any::Any;
/// use std::sync::Arc;
/// use rumi::{CustomMatchData, MatchingData};
///
/// #[derive(Debug)]
/// struct GeoLocation {
///     lat: f64,
///     lon: f64,
/// }
///
/// impl CustomMatchData for GeoLocation {
///     fn custom_type_name(&self) -> &'static str {
///         "geo_location"
///     }
///
///     fn as_any(&self) -> &dyn Any {
///         self
///     }
/// }
///
/// let data = MatchingData::Custom(Arc::new(GeoLocation { lat: 37.7749, lon: -122.4194 }));
/// assert!(data.is_custom());
/// assert_eq!(data.type_name(), "geo_location");
/// ```
pub trait CustomMatchData: Send + Sync + Debug {
    /// Returns a human-readable type identifier.
    ///
    /// Used for config-time validation to check if a `DataInput`
    /// produces compatible data for an `InputMatcher`.
    ///
    /// Convention: use `snake_case` names, e.g., `"geo_location"`, `"jwt_claims"`.
    ///
    /// Note: Named `custom_type_name` to avoid collision with `Any::type_id`.
    fn custom_type_name(&self) -> &'static str;

    /// Returns a reference to `self` as `&dyn Any`.
    ///
    /// Enables downcasting in `InputMatcher` implementations:
    ///
    /// ```ignore
    /// if let Some(geo) = custom.as_any().downcast_ref::<GeoLocation>() {
    ///     // use geo.lat, geo.lon
    /// }
    /// ```
    fn as_any(&self) -> &dyn Any;
}

/// The erased data type that flows between `DataInput` and `InputMatcher`.
///
/// Inspired by Envoy's `MatchingDataType = variant<monostate, string, shared_ptr<CustomMatchData>>`.
///
/// # Variants
///
/// - `Absent` — No data available (extractor returned nothing)
/// - `String` — String data (most common: headers, paths, query params)
/// - `Int` — Integer data
/// - `Bool` — Boolean data
/// - `Bytes` — Raw bytes data
/// - `Custom` — User-defined types implementing [`CustomMatchData`]
///
/// # Hybrid Design
///
/// Primitives stay stack-allocated (fast path), while `Custom` provides
/// extensibility via heap-allocated trait objects. This mirrors Envoy's
/// production-proven approach.
///
/// # Example
///
/// ```
/// use rumi::MatchingData;
///
/// let data = MatchingData::String("hello".to_string());
/// assert_eq!(data.as_str(), Some("hello"));
/// assert!(!data.is_absent());
/// ```
#[derive(Debug, Clone, Default)]
pub enum MatchingData {
    /// No data available (extractor returned nothing).
    /// When a predicate receives this, it evaluates to `false` (INV-1).
    #[default]
    Absent,

    /// String data — the most common case for HTTP headers, paths, etc.
    String(String),

    /// Integer data.
    Int(i64),

    /// Boolean data.
    Bool(bool),

    /// Raw bytes data.
    Bytes(Vec<u8>),

    /// Custom data type for domain-specific extensions.
    ///
    /// Wrap your [`CustomMatchData`] implementation with `Arc`:
    /// ```
    /// use std::sync::Arc;
    /// use rumi::{CustomMatchData, MatchingData};
    /// # use std::any::Any;
    /// # #[derive(Debug)] struct MyType;
    /// # impl CustomMatchData for MyType {
    /// #     fn custom_type_name(&self) -> &'static str { "my_type" }
    /// #     fn as_any(&self) -> &dyn Any { self }
    /// # }
    ///
    /// let data = MatchingData::Custom(Arc::new(MyType));
    /// ```
    Custom(Arc<dyn CustomMatchData>),
}

// Trait objects cannot derive `PartialEq`; `Custom` compares by allocation,
// because a custom type defines no other equality the engine can see.
impl PartialEq for MatchingData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Absent, Self::Absent) => true,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Bytes(a), Self::Bytes(b)) => a == b,
            (Self::Custom(a), Self::Custom(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl MatchingData {
    /// Returns `true` if this is the `Absent` variant.
    ///
    /// # Example
    ///
    /// ```
    /// use rumi::MatchingData;
    ///
    /// assert!(MatchingData::Absent.is_absent());
    /// assert!(!MatchingData::String("x".to_string()).is_absent());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    /// Returns `true` if this is the `String` variant.
    #[inline]
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Returns `true` if this is the `Int` variant.
    #[inline]
    #[must_use]
    pub fn is_int(&self) -> bool {
        matches!(self, Self::Int(_))
    }

    /// Returns `true` if this is the `Bool` variant.
    #[inline]
    #[must_use]
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    /// Returns `true` if this is the `Bytes` variant.
    #[inline]
    #[must_use]
    pub fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }

    /// Returns `true` if this is the `Custom` variant.
    #[inline]
    #[must_use]
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }

    /// Try to get the value as a string slice.
    ///
    /// # Example
    ///
    /// ```
    /// use rumi::MatchingData;
    ///
    /// let data = MatchingData::String("hello".to_string());
    /// assert_eq!(data.as_str(), Some("hello"));
    ///
    /// let data = MatchingData::Int(42);
    /// assert_eq!(data.as_str(), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => Option::None,
        }
    }

    /// Try to get the value as an integer.
    #[inline]
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => Option::None,
        }
    }

    /// Try to get the value as a boolean.
    #[inline]
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => Option::None,
        }
    }

    /// Try to get the value as a byte slice.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(b) => Some(b.as_slice()),
            _ => Option::None,
        }
    }

    /// Try to get the value as a custom match data reference.
    ///
    /// Returns a reference to the inner [`CustomMatchData`] trait object.
    /// Use [`CustomMatchData::as_any`] to downcast to the concrete type.
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::any::Any;
    /// use rumi::{CustomMatchData, MatchingData};
    ///
    /// #[derive(Debug)]
    /// struct MyData(i32);
    ///
    /// impl CustomMatchData for MyData {
    ///     fn custom_type_name(&self) -> &'static str { "my_data" }
    ///     fn as_any(&self) -> &dyn Any { self }
    /// }
    ///
    /// let data = MatchingData::Custom(Arc::new(MyData(42)));
    /// if let Some(custom) = data.as_custom() {
    ///     let my_data = custom.as_any().downcast_ref::<MyData>().unwrap();
    ///     assert_eq!(my_data.0, 42);
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn as_custom(&self) -> Option<&dyn CustomMatchData> {
        match self {
            Self::Custom(c) => Some(c.as_ref()),
            _ => Option::None,
        }
    }

    /// Returns a string describing the type of this data.
    ///
    /// Useful for config-time validation when checking if a `DataInput`
    /// produces compatible data for an `InputMatcher`.
    ///
    /// For `Custom` variants, this delegates to [`CustomMatchData::custom_type_name`].
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Absent => "none",
            Self::String(_) => "string",
            Self::Int(_) => "int",
            Self::Bool(_) => "bool",
            Self::Bytes(_) => "bytes",
            Self::Custom(c) => c.custom_type_name(),
        }
    }
}

impl From<String> for MatchingData {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for MatchingData {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for MatchingData {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<bool> for MatchingData {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<Vec<u8>> for MatchingData {
    fn from(b: Vec<u8>) -> Self {
        Self::Bytes(b)
    }
}

impl<T> From<Option<T>> for MatchingData
where
    T: Into<MatchingData>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            Option::None => Self::Absent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestCustomData {
        value: i32,
    }

    impl CustomMatchData for TestCustomData {
        fn custom_type_name(&self) -> &'static str {
            "test_custom"
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_is_none() {
        assert!(MatchingData::Absent.is_absent());
        assert!(!MatchingData::String("x".to_string()).is_absent());
        assert!(!MatchingData::Int(42).is_absent());
    }

    #[test]
    fn test_as_str() {
        let data = MatchingData::String("hello".to_string());
        assert_eq!(data.as_str(), Some("hello"));

        let data = MatchingData::Int(42);
        assert_eq!(data.as_str(), None);
    }

    #[test]
    fn test_from_conversions() {
        let data: MatchingData = "hello".into();
        assert!(matches!(data, MatchingData::String(_)));

        let data: MatchingData = 42i64.into();
        assert!(matches!(data, MatchingData::Int(42)));

        let data: MatchingData = true.into();
        assert!(matches!(data, MatchingData::Bool(true)));

        let data: MatchingData = Option::<String>::None.into();
        assert!(data.is_absent());

        let data: MatchingData = Some("hello".to_string()).into();
        assert_eq!(data.as_str(), Some("hello"));
    }

    #[test]
    fn test_type_name() {
        assert_eq!(MatchingData::Absent.type_name(), "none");
        assert_eq!(MatchingData::String("x".into()).type_name(), "string");
        assert_eq!(MatchingData::Int(1).type_name(), "int");
        assert_eq!(MatchingData::Bool(true).type_name(), "bool");
        assert_eq!(MatchingData::Bytes(vec![]).type_name(), "bytes");
    }

    #[test]
    fn test_custom_is_custom() {
        let custom = MatchingData::Custom(Arc::new(TestCustomData { value: 42 }));
        assert!(custom.is_custom());
        assert!(!custom.is_absent());
        assert!(!custom.is_string());

        assert!(!MatchingData::String("x".into()).is_custom());
        assert!(!MatchingData::Int(1).is_custom());
    }

    #[test]
    fn test_custom_as_custom_and_downcast() {
        let custom = MatchingData::Custom(Arc::new(TestCustomData { value: 42 }));

        let trait_obj = custom.as_custom().expect("should be Custom");
        assert_eq!(trait_obj.custom_type_name(), "test_custom");

        let concrete = trait_obj
            .as_any()
            .downcast_ref::<TestCustomData>()
            .expect("should downcast");
        assert_eq!(concrete.value, 42);

        assert!(MatchingData::String("x".into()).as_custom().is_none());
    }

    #[test]
    fn test_custom_type_name() {
        let custom = MatchingData::Custom(Arc::new(TestCustomData { value: 0 }));
        assert_eq!(custom.type_name(), "test_custom");
    }

    #[test]
    fn test_custom_clone() {
        let arc: Arc<dyn CustomMatchData> = Arc::new(TestCustomData { value: 99 });
        let data1 = MatchingData::Custom(Arc::clone(&arc));
        let data2 = data1.clone();

        assert!(Arc::ptr_eq(
            match &data1 {
                MatchingData::Custom(c) => c,
                _ => panic!(),
            },
            match &data2 {
                MatchingData::Custom(c) => c,
                _ => panic!(),
            }
        ));
    }

    #[test]
    fn test_custom_partial_eq() {
        let arc1: Arc<dyn CustomMatchData> = Arc::new(TestCustomData { value: 42 });
        let arc2: Arc<dyn CustomMatchData> = Arc::new(TestCustomData { value: 42 }); // Same value, different Arc

        let data1a = MatchingData::Custom(Arc::clone(&arc1));
        let data1b = MatchingData::Custom(Arc::clone(&arc1)); // Clone of same Arc
        let data2 = MatchingData::Custom(arc2);

        assert_eq!(data1a, data1b);

        assert_ne!(data1a, data2);

        assert_ne!(data1a, MatchingData::Int(42));
    }

    /// Integration test: Full extension story with custom types
    ///
    /// This test proves that users can extend the system without touching core:
    /// 1. Custom domain type (`GeoLocation`)
    /// 2. Custom context (`GeoRequest`)
    /// 3. Custom `DataInput` (`LocationInput`)
    /// 4. Custom `InputMatcher` (`WithinRadiusMatcher`)
    /// 5. Full Matcher pipeline with actions
    mod integration {
        use super::*;
        use crate::{
            DataInput, FieldMatcher, InputMatcher, Matcher, OnMatch, Predicate, SinglePredicate,
        };

        #[derive(Debug, Clone)]
        struct GeoLocation {
            lat: f64,
            lon: f64,
        }

        impl GeoLocation {
            fn distance_to(&self, other: &GeoLocation) -> f64 {
                ((self.lat - other.lat).powi(2) + (self.lon - other.lon).powi(2)).sqrt()
            }
        }

        impl CustomMatchData for GeoLocation {
            fn custom_type_name(&self) -> &'static str {
                "geo_location"
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        #[derive(Debug)]
        struct GeoRequest {
            user_location: GeoLocation,
        }

        #[derive(Debug)]
        struct LocationInput;

        impl DataInput<GeoRequest> for LocationInput {
            fn get(&self, ctx: &GeoRequest) -> MatchingData {
                MatchingData::Custom(Arc::new(ctx.user_location.clone()))
            }

            fn data_type(&self) -> &'static str {
                "geo_location"
            }
        }

        #[derive(Debug)]
        struct WithinRadiusMatcher {
            center: GeoLocation,
            radius: f64,
        }

        impl WithinRadiusMatcher {
            fn new(center: GeoLocation, radius: f64) -> Self {
                Self { center, radius }
            }
        }

        impl InputMatcher for WithinRadiusMatcher {
            fn matches(&self, value: &MatchingData) -> bool {
                value
                    .as_custom()
                    .and_then(|c| c.as_any().downcast_ref::<GeoLocation>())
                    .is_some_and(|loc| loc.distance_to(&self.center) <= self.radius)
            }

            fn supported_types(&self) -> &[&'static str] {
                &["geo_location"]
            }
        }

        #[test]
        fn test_full_custom_extension_pipeline() {
            let sf_center = GeoLocation {
                lat: 37.7749,
                lon: -122.4194,
            };
            let nyc_center = GeoLocation {
                lat: 40.7128,
                lon: -74.0060,
            };

            let matcher: Matcher<GeoRequest, String> = Matcher::list(
                vec![
                    FieldMatcher::new(
                        Predicate::Single(SinglePredicate::new(
                            Box::new(LocationInput),
                            Box::new(WithinRadiusMatcher::new(sf_center.clone(), 1.0)),
                        )),
                        OnMatch::Action("sf_backend".to_string()),
                    ),
                    FieldMatcher::new(
                        Predicate::Single(SinglePredicate::new(
                            Box::new(LocationInput),
                            Box::new(WithinRadiusMatcher::new(nyc_center.clone(), 1.0)),
                        )),
                        OnMatch::Action("nyc_backend".to_string()),
                    ),
                ],
                Some(OnMatch::Action("default_backend".to_string())),
            );

            let sf_request = GeoRequest {
                user_location: GeoLocation {
                    lat: 37.78,
                    lon: -122.42,
                },
            };
            assert_eq!(
                matcher.evaluate(&sf_request),
                Some("sf_backend".to_string())
            );

            let nyc_request = GeoRequest {
                user_location: GeoLocation {
                    lat: 40.71,
                    lon: -74.01,
                },
            };
            assert_eq!(
                matcher.evaluate(&nyc_request),
                Some("nyc_backend".to_string())
            );

            let london_request = GeoRequest {
                user_location: GeoLocation {
                    lat: 51.5074,
                    lon: -0.1278,
                },
            };
            assert_eq!(
                matcher.evaluate(&london_request),
                Some("default_backend".to_string())
            );
        }
    }
}
