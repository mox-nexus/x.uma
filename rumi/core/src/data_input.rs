//! `DataInput` — Domain-specific data extraction
//!
//! The `DataInput` trait extracts data from a domain-specific context (e.g., HTTP request,
//! Claude hook context) and returns type-erased `MatchingData`.
//!
//! This is generic over the context type `Ctx`, but returns domain-agnostic `MatchingData`,
//! enabling `InputMatchers` to be shared across different domains.

use crate::MatchingData;
use std::fmt::Debug;

/// Extracts data from a domain-specific context.
///
/// `DataInput` is the bridge between domain-specific contexts (like HTTP requests)
/// and domain-agnostic matchers. It extracts relevant data from the context and
/// returns it as type-erased [`MatchingData`].
///
/// # Type Parameters
///
/// - `Ctx`: The context type this input operates on (e.g., `HttpRequest`, `HookContext`)
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to support concurrent evaluation
/// and FFI use cases.
///
/// # Example
///
/// ```ignore
/// use rumi::{DataInput, MatchingData};
///
/// #[derive(Debug)]
/// struct HeaderInput { name: String }
///
/// impl DataInput<HttpRequest> for HeaderInput {
///     fn get(&self, ctx: &HttpRequest) -> MatchingData {
///         ctx.headers.get(&self.name)
///             .map(|v| MatchingData::String(v.clone()))
///             .unwrap_or(MatchingData::None)
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `DataInput<{Ctx}>`",
    label = "this type cannot extract data from `{Ctx}`",
    note = "DataInput<Ctx> extracts data from a specific context type",
    note = "ensure your input type implements DataInput for the correct context (e.g., DataInput<HttpRequest>, DataInput<HookContext>)"
)]
pub trait DataInput<Ctx>: Send + Sync + Debug {
    /// Extract data from the given context.
    ///
    /// Returns [`MatchingData::None`] if the requested data is not present.
    /// This is important for the INV (Dijkstra): None → predicate evaluates to false.
    fn get(&self, ctx: &Ctx) -> MatchingData;

    /// Returns a static string describing the type of data this input produces.
    ///
    /// Used for config-time validation to ensure DataInput/InputMatcher compatibility.
    /// Default is `"string"` since most inputs produce string data.
    fn data_type(&self) -> &'static str {
        "string"
    }
}

// Blanket implementation for boxed DataInputs
#[diagnostic::do_not_recommend]
impl<Ctx> DataInput<Ctx> for Box<dyn DataInput<Ctx>> {
    fn get(&self, ctx: &Ctx) -> MatchingData {
        (**self).get(ctx)
    }

    fn data_type(&self) -> &'static str {
        (**self).data_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestContext {
        value: String,
    }

    #[derive(Debug)]
    struct ValueInput;

    impl DataInput<TestContext> for ValueInput {
        fn get(&self, ctx: &TestContext) -> MatchingData {
            MatchingData::String(ctx.value.clone())
        }
    }

    #[test]
    fn test_data_input_basic() {
        let ctx = TestContext {
            value: "hello".to_string(),
        };
        let input = ValueInput;
        let data = input.get(&ctx);
        assert_eq!(data.as_str(), Some("hello"));
    }

    #[test]
    fn test_data_input_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn DataInput<TestContext>>>();
    }
}
