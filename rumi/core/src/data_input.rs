//! `DataInput` — Domain-specific data extraction
//!
//! The `DataInput` trait extracts data from a domain-specific context (e.g., HTTP request,
//! Claude hook context) and returns type-erased `MatchingData`.
//!
//! This is generic over the context type `Ctx`, but returns domain-agnostic `MatchingData`,
//! enabling `InputMatchers` to be shared across different domains.

use crate::MatchingData;
use std::borrow::Cow;
use std::fmt::Debug;

/// Extracts data from a domain-specific context.
///
/// `DataInput` is the bridge between domain-specific contexts (like HTTP requests)
/// and domain-agnostic matchers. It extracts relevant data from the context and
/// returns it as type-erased [`MatchingData`].
///
/// # Type Parameters
///
/// - `Ctx`: The context type this input operates on (e.g., `HttpMessage`, `HookContext`)
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
/// impl DataInput<HttpMessage> for HeaderInput {
///     fn get(&self, ctx: &HttpMessage) -> Option<MatchingData> {
///         ctx.headers.get(&self.name).map(|v| MatchingData::String(v.clone()))
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `DataInput<{Ctx}>`",
    label = "this type cannot extract data from `{Ctx}`",
    note = "DataInput<Ctx> extracts data from a specific context type",
    note = "ensure your input type implements DataInput for the correct context (e.g., DataInput<HttpMessage>, DataInput<HookContext>)"
)]
pub trait DataInput<Ctx>: Send + Sync + Debug {
    /// Extract data from the given context.
    ///
    /// `None` means the context has no such value. The predicate is then
    /// `false` and the matcher is never consulted (INV-1); that is enforced by
    /// this signature, not by every implementation remembering it.
    ///
    /// # Purity
    ///
    /// Same context, same result, and no effect observable by another call.
    /// The engine may call `get` more or fewer times than there are rules, in
    /// any order, and may share one call between rules that read the same
    /// input (D-062).
    fn get(&self, ctx: &Ctx) -> Option<MatchingData>;

    /// Returns a static string describing the type of data this input produces.
    ///
    /// Used for config-time validation to ensure DataInput/InputMatcher compatibility.
    /// Default is `"string"` since most inputs produce string data.
    fn data_type(&self) -> &'static str {
        "string"
    }

    /// What this input reads, as a name: two inputs with equal identities read
    /// the same value from any context.
    ///
    /// `None`, the default, keeps the input out of any evaluation that shares
    /// one extraction between rules. An input that is impure, or whose
    /// sameness is not decidable from its configuration, returns `None` (D-062).
    fn identity(&self) -> Option<Cow<'_, str>> {
        None
    }
}

#[diagnostic::do_not_recommend]
impl<Ctx> DataInput<Ctx> for Box<dyn DataInput<Ctx>> {
    fn get(&self, ctx: &Ctx) -> Option<MatchingData> {
        (**self).get(ctx)
    }

    fn data_type(&self) -> &'static str {
        (**self).data_type()
    }

    fn identity(&self) -> Option<Cow<'_, str>> {
        (**self).identity()
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
        fn get(&self, ctx: &TestContext) -> Option<MatchingData> {
            Some(MatchingData::String(ctx.value.clone()))
        }
    }

    #[test]
    fn test_data_input_basic() {
        let ctx = TestContext {
            value: "hello".to_string(),
        };
        let input = ValueInput;
        let data = input.get(&ctx).expect("value is set");
        assert_eq!(data.as_str(), Some("hello"));
    }
}
