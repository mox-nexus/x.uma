//! `MatchingData` — Type-erased data that flows between `DataInput` and `InputMatcher`
//!
//! This is the key insight from Envoy's design: type erasure at the data level.
//! `DataInputs` produce `MatchingData`, and `InputMatchers` consume it.
//! This allows `InputMatchers` to be non-generic and shareable across contexts.

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{string::String, string::ToString, vec::Vec};

#[cfg(feature = "std")]
use std::string::String;
#[cfg(feature = "std")]
use std::vec::Vec;

/// The erased data type that flows between `DataInput` and `InputMatcher`.
///
/// Inspired by Envoy's `MatchingDataType = variant<monostate, string, ...>`.
///
/// # Variants
///
/// - `None` — No data available (extractor returned nothing)
/// - `String` — String data (most common: headers, paths, query params)
/// - `Int` — Integer data
/// - `Bool` — Boolean data
/// - `Bytes` — Raw bytes data
///
/// # Example
///
/// ```
/// use rumi_core::MatchingData;
///
/// let data = MatchingData::String("hello".to_string());
/// assert_eq!(data.as_str(), Some("hello"));
/// assert!(!data.is_none());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum MatchingData {
    /// No data available (extractor returned nothing).
    /// When a predicate receives this, it evaluates to `false` (INV: Dijkstra).
    None,

    /// String data — the most common case for HTTP headers, paths, etc.
    String(String),

    /// Integer data.
    Int(i64),

    /// Boolean data.
    Bool(bool),

    /// Raw bytes data.
    Bytes(Vec<u8>),
}

impl MatchingData {
    /// Returns `true` if this is the `None` variant.
    ///
    /// # Example
    ///
    /// ```
    /// use rumi_core::MatchingData;
    ///
    /// assert!(MatchingData::None.is_none());
    /// assert!(!MatchingData::String("x".to_string()).is_none());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
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

    /// Try to get the value as a string slice.
    ///
    /// # Example
    ///
    /// ```
    /// use rumi_core::MatchingData;
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

    /// Returns a static string describing the type of this data.
    ///
    /// Useful for config-time validation when checking if a `DataInput`
    /// produces compatible data for an `InputMatcher`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::String(_) => "string",
            Self::Int(_) => "int",
            Self::Bool(_) => "bool",
            Self::Bytes(_) => "bytes",
        }
    }
}

impl Default for MatchingData {
    fn default() -> Self {
        Self::None
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
            Option::None => Self::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_none() {
        assert!(MatchingData::None.is_none());
        assert!(!MatchingData::String("x".to_string()).is_none());
        assert!(!MatchingData::Int(42).is_none());
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
        assert!(data.is_none());

        let data: MatchingData = Some("hello".to_string()).into();
        assert_eq!(data.as_str(), Some("hello"));
    }

    #[test]
    fn test_type_name() {
        assert_eq!(MatchingData::None.type_name(), "none");
        assert_eq!(MatchingData::String("x".into()).type_name(), "string");
        assert_eq!(MatchingData::Int(1).type_name(), "int");
        assert_eq!(MatchingData::Bool(true).type_name(), "bool");
        assert_eq!(MatchingData::Bytes(vec![]).type_name(), "bytes");
    }
}
