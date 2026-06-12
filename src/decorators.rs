//! Decorator-like convenience helpers.
//!
//! Python Click exposes a decorator factory `make_pass_decorator()` that creates
//! decorators to pass a context object (stored on `Context.obj`) into callbacks.
//!
//! Rust doesn't have function decorators, so click-rs provides a small adapter
//! that converts a function `Fn(&T, &Context)` into a standard `CommandCallback`
//! (`Fn(&Context)`), looking up `T` from the context.

use std::marker::PhantomData;

use crate::command::CommandCallback;
use crate::context::Context;
use crate::error::{ClickError, Result};

/// A decorator factory for passing a context object of type `T` to a callback.
///
/// Create one with [`make_pass_decorator`], then call [`PassDecorator::decorate`]
/// to wrap a function that expects `&T`.
///
/// This matches the most common Click usage (ensure = false). Unlike Python,
/// click-rs callbacks receive `&Context` (not `&mut Context`), so this helper
/// does not support "ensure" semantics that would create and store an object
/// if it is missing.
pub struct PassDecorator<T> {
    _marker: PhantomData<T>,
}

/// Create a pass-decorator for `T`.
///
/// # Example
///
/// ```rust
/// use click::{make_pass_decorator, ContextBuilder};
///
/// #[derive(Debug)]
/// struct AppState {
///     value: i32,
/// }
///
/// let cb = make_pass_decorator::<AppState>().decorate(|state, _ctx| {
///     assert_eq!(state.value, 123);
///     Ok(())
/// });
///
/// let ctx = ContextBuilder::new().obj(AppState { value: 123 }).build();
/// cb(&ctx).unwrap();
/// ```
pub fn make_pass_decorator<T: 'static>() -> PassDecorator<T> {
    PassDecorator {
        _marker: PhantomData,
    }
}

impl<T: 'static> PassDecorator<T> {
    /// Wrap a function that expects `&T` into a click-rs [`CommandCallback`].
    pub fn decorate<F>(self, f: F) -> CommandCallback
    where
        F: Fn(&T, &Context) -> Result<()> + Send + Sync + 'static,
    {
        Box::new(move |ctx: &Context| {
            let obj = ctx.find_obj::<T>().ok_or_else(|| {
                ClickError::usage("Missing context object for make_pass_decorator().")
            })?;
            f(obj, ctx)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextBuilder;
    use std::sync::Arc;

    #[derive(Debug)]
    struct AppState {
        value: i32,
    }

    #[test]
    fn test_make_pass_decorator_passes_obj() {
        let cb = make_pass_decorator::<AppState>().decorate(|state, _ctx| {
            assert_eq!(state.value, 7);
            Ok(())
        });

        let ctx = ContextBuilder::new().obj(AppState { value: 7 }).build();
        cb(&ctx).unwrap();
    }

    #[test]
    fn test_make_pass_decorator_finds_obj_from_parent() {
        let cb = make_pass_decorator::<AppState>().decorate(|state, _ctx| {
            assert_eq!(state.value, 9);
            Ok(())
        });

        let parent = Arc::new(ContextBuilder::new().obj(AppState { value: 9 }).build());
        let child = ContextBuilder::new().parent(parent).build();
        cb(&child).unwrap();
    }
}
