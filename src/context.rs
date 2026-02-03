//! Central execution context for click-rs.
//!
//! The [`Context`] struct holds state relevant for script execution at every level.
//! It manages parameter values, parent-child relationships, and provides access to
//! shared metadata and resources.
//!
//! # Thread-Local Context Stack
//!
//! Click uses a thread-local stack to provide implicit access to the current context.
//! Use [`push_context`], [`pop_context`], and [`get_current_context`] to manage this stack.
//!
//! # Example
//!
//! ```
//! use click::context::{Context, ContextBuilder, push_context, pop_context, get_current_context};
//! use std::sync::Arc;
//!
//! let ctx = ContextBuilder::new()
//!     .info_name("myapp")
//!     .build();
//! let ctx = Arc::new(ctx);
//!
//! push_context(Arc::clone(&ctx));
//! assert!(get_current_context().is_some());
//! pop_context();
//! assert!(get_current_context().is_none());
//! ```

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::ClickError;
use crate::source::ParameterSource;

// Thread-local context stack
thread_local! {
    static CONTEXT_STACK: RefCell<Vec<Arc<Context>>> = const { RefCell::new(Vec::new()) };
}

/// Push a context onto the thread-local context stack.
///
/// The pushed context becomes the current context accessible via [`get_current_context`].
///
/// # Example
///
/// ```
/// use click::context::{Context, ContextBuilder, push_context, get_current_context, pop_context};
/// use std::sync::Arc;
///
/// let ctx = Arc::new(ContextBuilder::new().info_name("myapp").build());
/// push_context(Arc::clone(&ctx));
///
/// let current = get_current_context().unwrap();
/// assert_eq!(current.info_name(), Some("myapp"));
///
/// pop_context();
/// ```
pub fn push_context(ctx: Arc<Context>) {
    CONTEXT_STACK.with(|stack| {
        stack.borrow_mut().push(ctx);
    });
}

/// Pop and return the top context from the thread-local context stack.
///
/// Returns `None` if the stack is empty.
///
/// # Example
///
/// ```
/// use click::context::{Context, ContextBuilder, push_context, pop_context};
/// use std::sync::Arc;
///
/// let ctx = Arc::new(ContextBuilder::new().build());
/// push_context(ctx);
///
/// let popped = pop_context();
/// assert!(popped.is_some());
/// assert!(pop_context().is_none());
/// ```
pub fn pop_context() -> Option<Arc<Context>> {
    CONTEXT_STACK.with(|stack| stack.borrow_mut().pop())
}

/// Get the current context from the thread-local stack.
///
/// Returns `None` if no context is currently active. Use [`push_context`] to
/// make a context current.
///
/// # Example
///
/// ```
/// use click::context::{Context, ContextBuilder, push_context, pop_context, get_current_context};
/// use std::sync::Arc;
///
/// assert!(get_current_context().is_none());
///
/// let ctx = Arc::new(ContextBuilder::new().info_name("cli").build());
/// push_context(ctx);
///
/// let current = get_current_context().expect("context should be available");
/// assert_eq!(current.info_name(), Some("cli"));
///
/// pop_context();
/// ```
pub fn get_current_context() -> Option<Arc<Context>> {
    CONTEXT_STACK.with(|stack| stack.borrow().last().cloned())
}

/// Type alias for shared values that can be stored in context maps.
/// Uses Arc to allow cloning and sharing across parent-child contexts.
pub type BoxedValue = Arc<dyn Any + Send + Sync>;

/// The central execution context for click-rs.
///
/// The context holds state relevant for script execution at every single level.
/// It's normally invisible to commands unless they opt-in to getting access to it.
///
/// A context manages:
/// - Parsed parameter values
/// - Parent context chain (for nested commands)
/// - Parameter source tracking (CLI, env, default, prompt)
/// - Resource cleanup via close callbacks
/// - Shared metadata between nested contexts
///
/// # Construction
///
/// Use [`ContextBuilder`] to create a new context with custom settings.
///
/// # Example
///
/// ```
/// use click::context::{Context, ContextBuilder};
/// use std::sync::Arc;
///
/// let parent = Arc::new(ContextBuilder::new()
///     .info_name("cli")
///     .auto_envvar_prefix("MYAPP")
///     .build());
///
/// let child = ContextBuilder::new()
///     .info_name("subcommand")
///     .parent(parent)
///     .build();
///
/// assert_eq!(child.command_path(), "cli subcommand");
/// ```
pub struct Context {
    /// The parent context, if any.
    parent: Option<Arc<Context>>,

    /// The descriptive information name for this invocation.
    /// For the toplevel script it's usually the name of the script;
    /// for subcommands, it's the command name.
    info_name: Option<String>,

    /// Map of parameter names to their parsed values.
    params: HashMap<String, BoxedValue>,

    /// Leftover arguments that were not consumed by parameters.
    args: Vec<String>,

    /// User data object that can be passed between commands.
    obj: Option<BoxedValue>,

    /// Shared metadata dictionary accessible to all nested contexts.
    /// Keys should be unique dotted strings (e.g., module paths).
    meta: HashMap<String, BoxedValue>,

    /// A dictionary with default overrides for parameters.
    default_map: Option<HashMap<String, BoxedValue>>,

    /// This flag indicates if a subcommand is going to be executed.
    /// `None` means no subcommand; `Some("*")` for chained commands;
    /// otherwise the name of the subcommand to execute.
    invoked_subcommand: Option<String>,

    /// The width of the terminal (None for autodetection).
    terminal_width: Option<usize>,

    /// The maximum width for content rendered by Click (e.g., help pages).
    /// Defaults to 80 characters if not overridden.
    max_content_width: Option<usize>,

    /// If true, extra arguments at the end will not raise an error.
    allow_extra_args: bool,

    /// If false, options and arguments cannot be mixed.
    allow_interspersed_args: bool,

    /// If true, unknown options are ignored and kept for later processing.
    ignore_unknown_options: bool,

    /// The names for the help options. Default is `["--help"]`.
    help_option_names: Vec<String>,

    /// If true, Click will parse without interactivity or callback invocation.
    /// Default values will also be ignored. Useful for shell completion.
    resilient_parsing: bool,

    /// The prefix to use for automatic environment variables.
    /// If `None`, reading from environment variables is disabled.
    auto_envvar_prefix: Option<String>,

    /// Controls if styling output is wanted or not.
    color: Option<bool>,

    /// Show option default values when formatting help text.
    show_default: Option<bool>,

    /// Map of parameter names to their value sources.
    parameter_source: HashMap<String, ParameterSource>,

    /// Callbacks to run when the context is closed.
    /// These are stored in a RefCell to allow mutation even with shared references.
    close_callbacks: RefCell<Vec<Box<dyn FnOnce() + Send>>>,
}

// Manual Debug implementation since close_callbacks contains non-Debug closures
impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field(
                "parent",
                &self.parent.as_ref().map(|p| p.info_name.as_deref()),
            )
            .field("info_name", &self.info_name)
            .field("params", &format!("<{} params>", self.params.len()))
            .field("args", &self.args)
            .field("obj", &self.obj.as_ref().map(|_| "<obj>"))
            .field("meta", &format!("<{} entries>", self.meta.len()))
            .field(
                "default_map",
                &self
                    .default_map
                    .as_ref()
                    .map(|m| format!("<{} entries>", m.len())),
            )
            .field("invoked_subcommand", &self.invoked_subcommand)
            .field("terminal_width", &self.terminal_width)
            .field("max_content_width", &self.max_content_width)
            .field("allow_extra_args", &self.allow_extra_args)
            .field("allow_interspersed_args", &self.allow_interspersed_args)
            .field("ignore_unknown_options", &self.ignore_unknown_options)
            .field("help_option_names", &self.help_option_names)
            .field("resilient_parsing", &self.resilient_parsing)
            .field("auto_envvar_prefix", &self.auto_envvar_prefix)
            .field("color", &self.color)
            .field("show_default", &self.show_default)
            .field("parameter_source", &self.parameter_source)
            .field(
                "close_callbacks",
                &format!("<{} callbacks>", self.close_callbacks.borrow().len()),
            )
            .finish()
    }
}

// Manual Default implementation since we can't derive it due to the close_callbacks field
impl Default for Context {
    fn default() -> Self {
        Self {
            parent: None,
            info_name: None,
            params: HashMap::new(),
            args: Vec::new(),
            obj: None,
            meta: HashMap::new(),
            default_map: None,
            invoked_subcommand: None,
            terminal_width: None,
            max_content_width: None,
            allow_extra_args: false,
            allow_interspersed_args: true,
            ignore_unknown_options: false,
            help_option_names: vec!["--help".to_string()],
            resilient_parsing: false,
            auto_envvar_prefix: None,
            color: None,
            show_default: None,
            parameter_source: HashMap::new(),
            close_callbacks: RefCell::new(Vec::new()),
        }
    }
}

impl Context {
    /// Create a new context with default settings.
    ///
    /// For more control over the context's configuration, use [`ContextBuilder`].
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the parent context, if any.
    #[inline]
    pub fn parent(&self) -> Option<&Arc<Context>> {
        self.parent.as_ref()
    }

    /// Get the info name for this context.
    #[inline]
    pub fn info_name(&self) -> Option<&str> {
        self.info_name.as_deref()
    }

    /// Get the parsed parameter values.
    #[inline]
    pub fn params(&self) -> &HashMap<String, BoxedValue> {
        &self.params
    }

    /// Get a mutable reference to the parsed parameter values.
    #[inline]
    pub fn params_mut(&mut self) -> &mut HashMap<String, BoxedValue> {
        &mut self.params
    }

    /// Get a parameter value by name, downcast to the expected type.
    ///
    /// Returns `None` if the parameter doesn't exist or has the wrong type.
    ///
    /// # Example
    ///
    /// ```
    /// use click::context::ContextBuilder;
    /// use std::sync::Arc;
    ///
    /// let mut ctx = ContextBuilder::new().build();
    /// ctx.params_mut().insert("count".to_string(), Arc::new(42i32));
    ///
    /// let count: Option<&i32> = ctx.get_param("count");
    /// assert_eq!(count, Some(&42));
    /// ```
    pub fn get_param<T: 'static>(&self, name: &str) -> Option<&T> {
        self.params.get(name).and_then(|v| v.downcast_ref::<T>())
    }

    /// Get the leftover arguments.
    #[inline]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Get a mutable reference to the leftover arguments.
    #[inline]
    pub fn args_mut(&mut self) -> &mut Vec<String> {
        &mut self.args
    }

    /// Get the user object, downcast to the expected type.
    ///
    /// # Example
    ///
    /// ```
    /// use click::context::ContextBuilder;
    ///
    /// #[derive(Debug)]
    /// struct AppState { db_url: String }
    ///
    /// let ctx = ContextBuilder::new()
    ///     .obj(AppState { db_url: "postgres://...".to_string() })
    ///     .build();
    ///
    /// let state: Option<&AppState> = ctx.obj();
    /// assert!(state.is_some());
    /// ```
    pub fn obj<T: 'static>(&self) -> Option<&T> {
        self.obj.as_ref().and_then(|v| v.downcast_ref::<T>())
    }

    /// Find a user object of type `T` by walking up the parent chain.
    ///
    /// This is a convenience for decorator-style helpers that want to accept
    /// objects stored in a parent context.
    pub fn find_obj<T: 'static>(&self) -> Option<&T> {
        let mut current: Option<&Context> = Some(self);
        while let Some(ctx) = current {
            if let Some(obj) = ctx.obj::<T>() {
                return Some(obj);
            }
            current = ctx.parent.as_ref().map(|p| p.as_ref());
        }
        None
    }

    /// Set the user object.
    pub fn set_obj<T: Any + Send + Sync + 'static>(&mut self, obj: T) {
        self.obj = Some(Arc::new(obj));
    }

    /// Get the shared metadata map.
    #[inline]
    pub fn meta(&self) -> &HashMap<String, BoxedValue> {
        &self.meta
    }

    /// Get a mutable reference to the shared metadata map.
    #[inline]
    pub fn meta_mut(&mut self) -> &mut HashMap<String, BoxedValue> {
        &mut self.meta
    }

    /// Get a metadata value by key, downcast to the expected type.
    pub fn get_meta<T: 'static>(&self, key: &str) -> Option<&T> {
        self.meta.get(key).and_then(|v| v.downcast_ref::<T>())
    }

    /// Get the invoked subcommand name, if any.
    #[inline]
    pub fn invoked_subcommand(&self) -> Option<&str> {
        self.invoked_subcommand.as_deref()
    }

    /// Set the invoked subcommand name.
    #[inline]
    pub fn set_invoked_subcommand(&mut self, name: Option<String>) {
        self.invoked_subcommand = name;
    }

    /// Get the terminal width.
    #[inline]
    pub fn terminal_width(&self) -> Option<usize> {
        self.terminal_width
    }

    /// Get the maximum content width.
    #[inline]
    pub fn max_content_width(&self) -> Option<usize> {
        self.max_content_width
    }

    /// Check if extra arguments are allowed.
    #[inline]
    pub fn allow_extra_args(&self) -> bool {
        self.allow_extra_args
    }

    /// Check if interspersed arguments are allowed.
    #[inline]
    pub fn allow_interspersed_args(&self) -> bool {
        self.allow_interspersed_args
    }

    /// Check if unknown options should be ignored.
    #[inline]
    pub fn ignore_unknown_options(&self) -> bool {
        self.ignore_unknown_options
    }

    /// Get the help option names.
    #[inline]
    pub fn help_option_names(&self) -> &[String] {
        &self.help_option_names
    }

    /// Check if resilient parsing is enabled.
    #[inline]
    pub fn resilient_parsing(&self) -> bool {
        self.resilient_parsing
    }

    /// Get the auto envvar prefix.
    #[inline]
    pub fn auto_envvar_prefix(&self) -> Option<&str> {
        self.auto_envvar_prefix.as_deref()
    }

    /// Get the color setting.
    #[inline]
    pub fn color(&self) -> Option<bool> {
        self.color
    }

    /// Get the show_default setting.
    #[inline]
    pub fn show_default(&self) -> Option<bool> {
        self.show_default
    }

    /// Get the computed command path.
    ///
    /// This is used for the `usage` information on the help page.
    /// It's automatically created by combining the info names of the
    /// chain of contexts to the root.
    ///
    /// # Example
    ///
    /// ```
    /// use click::context::ContextBuilder;
    /// use std::sync::Arc;
    ///
    /// let root = Arc::new(ContextBuilder::new().info_name("cli").build());
    /// let sub = ContextBuilder::new()
    ///     .info_name("subcommand")
    ///     .parent(root)
    ///     .build();
    ///
    /// assert_eq!(sub.command_path(), "cli subcommand");
    /// ```
    pub fn command_path(&self) -> String {
        let mut parts = Vec::new();

        // Walk up the parent chain to collect all info names
        let mut current: Option<&Context> = Some(self);
        while let Some(ctx) = current {
            if let Some(name) = &ctx.info_name {
                parts.push(name.as_str());
            }
            current = ctx.parent.as_ref().map(|p| p.as_ref());
        }

        // Reverse to get root-to-leaf order
        parts.reverse();
        parts.join(" ")
    }

    /// Find the root context.
    ///
    /// Walks up the parent chain to find the outermost context.
    pub fn find_root(&self) -> &Context {
        let mut current: &Context = self;
        while let Some(parent) = &current.parent {
            current = parent.as_ref();
        }
        current
    }

    /// Get the source of a parameter's value.
    ///
    /// Returns `None` if the parameter was not provided from any source.
    ///
    /// # Example
    ///
    /// ```
    /// use click::context::ContextBuilder;
    /// use click::ParameterSource;
    ///
    /// let mut ctx = ContextBuilder::new().build();
    /// ctx.set_parameter_source("name", ParameterSource::CommandLine);
    ///
    /// assert_eq!(ctx.get_parameter_source("name"), Some(ParameterSource::CommandLine));
    /// assert_eq!(ctx.get_parameter_source("other"), None);
    /// ```
    #[inline]
    pub fn get_parameter_source(&self, name: &str) -> Option<ParameterSource> {
        self.parameter_source.get(name).copied()
    }

    /// Set the source of a parameter's value.
    ///
    /// This indicates the location from which the value of the parameter was obtained.
    #[inline]
    pub fn set_parameter_source(&mut self, name: &str, source: ParameterSource) {
        self.parameter_source.insert(name.to_string(), source);
    }

    /// Create a usage error for this context.
    ///
    /// Returns a [`ClickError::UsageError`] with the given message.
    ///
    /// # Example
    ///
    /// ```
    /// use click::context::ContextBuilder;
    /// use click::error::ClickError;
    ///
    /// let ctx = ContextBuilder::new().info_name("myapp").build();
    /// let err = ctx.fail("invalid input");
    ///
    /// assert!(matches!(err, ClickError::UsageError { .. }));
    /// ```
    pub fn fail(&self, message: &str) -> ClickError {
        ClickError::usage(message)
    }

    /// Create an abort error.
    ///
    /// Returns a [`ClickError::Abort`].
    ///
    /// # Example
    ///
    /// ```
    /// use click::context::ContextBuilder;
    /// use click::error::ClickError;
    ///
    /// let ctx = ContextBuilder::new().build();
    /// let err = ctx.abort();
    ///
    /// assert!(matches!(err, ClickError::Abort));
    /// ```
    #[inline]
    pub fn abort(&self) -> ClickError {
        ClickError::abort()
    }

    /// Create an exit error with the given code.
    ///
    /// Returns a [`ClickError::Exit`] with the specified exit code.
    ///
    /// # Example
    ///
    /// ```
    /// use click::context::ContextBuilder;
    /// use click::error::ClickError;
    ///
    /// let ctx = ContextBuilder::new().build();
    /// let err = ctx.exit(1);
    ///
    /// assert!(matches!(err, ClickError::Exit { code: 1 }));
    /// ```
    #[inline]
    pub fn exit(&self, code: i32) -> ClickError {
        ClickError::exit(code)
    }

    /// Register a callback to be called when the context is closed.
    ///
    /// This can be used to close resources opened during script execution.
    ///
    /// # Example
    ///
    /// ```
    /// use click::context::ContextBuilder;
    /// use std::sync::atomic::{AtomicBool, Ordering};
    /// use std::sync::Arc;
    ///
    /// let closed = Arc::new(AtomicBool::new(false));
    /// let closed_clone = Arc::clone(&closed);
    ///
    /// let mut ctx = ContextBuilder::new().build();
    /// ctx.call_on_close(move || {
    ///     closed_clone.store(true, Ordering::SeqCst);
    /// });
    ///
    /// assert!(!closed.load(Ordering::SeqCst));
    /// ctx.close();
    /// assert!(closed.load(Ordering::SeqCst));
    /// ```
    pub fn call_on_close(&self, f: impl FnOnce() + Send + 'static) {
        self.close_callbacks.borrow_mut().push(Box::new(f));
    }

    /// Invoke all close callbacks.
    ///
    /// This runs all functions registered with [`call_on_close`](Self::call_on_close)
    /// in reverse order (last registered, first called).
    pub fn close(&self) {
        let callbacks: Vec<_> = self.close_callbacks.borrow_mut().drain(..).collect();
        // Run in reverse order (LIFO)
        for callback in callbacks.into_iter().rev() {
            callback();
        }
    }

    /// Look up a default value for a parameter from the default_map.
    ///
    /// Returns `None` if no default_map is set or the parameter is not in the map.
    ///
    /// # Example
    ///
    /// ```
    /// use click::context::ContextBuilder;
    /// use std::collections::HashMap;
    /// use std::sync::Arc;
    ///
    /// let mut defaults: HashMap<String, Arc<dyn std::any::Any + Send + Sync>> = HashMap::new();
    /// defaults.insert("count".to_string(), Arc::new(10i32));
    ///
    /// let ctx = ContextBuilder::new()
    ///     .default_map(defaults)
    ///     .build();
    ///
    /// let default = ctx.lookup_default("count");
    /// assert!(default.is_some());
    /// assert_eq!(default.and_then(|v| v.downcast_ref::<i32>()), Some(&10));
    /// ```
    pub fn lookup_default(&self, name: &str) -> Option<&dyn Any> {
        self.default_map
            .as_ref()
            .and_then(|map| map.get(name))
            .map(|v| v.as_ref() as &dyn Any)
    }

    /// Programmatically invoke another command with the given arguments.
    ///
    /// This creates a child context for the invoked command and runs it.
    /// The invoked command will have this context as its parent.
    ///
    /// This is useful for calling other commands from within a command callback,
    /// similar to Python Click's `ctx.invoke()`.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command to invoke
    /// * `args` - The arguments to pass to the command
    ///
    /// # Example
    ///
    /// ```
    /// use click::command::Command;
    /// use click::context::ContextBuilder;
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicBool, Ordering};
    ///
    /// let invoked = Arc::new(AtomicBool::new(false));
    /// let invoked_clone = Arc::clone(&invoked);
    ///
    /// let other_cmd = Command::new("other")
    ///     .callback(move |_ctx| {
    ///         invoked_clone.store(true, Ordering::SeqCst);
    ///         Ok(())
    ///     })
    ///     .build();
    ///
    /// let ctx = Arc::new(ContextBuilder::new().info_name("main").build());
    /// let result = ctx.invoke(&other_cmd, &[]);
    /// assert!(result.is_ok());
    /// assert!(invoked.load(Ordering::SeqCst));
    /// ```
    pub fn invoke(
        self: &Arc<Self>,
        cmd: &dyn crate::group::CommandLike,
        args: &[String],
    ) -> Result<(), ClickError> {
        // Get the command name for the child context
        let cmd_name = cmd.name().unwrap_or("invoked");

        // Create child context with this context as parent
        let child_ctx = cmd.make_context(cmd_name, args.to_vec(), Some(Arc::clone(self)))?;
        let child_ctx = Arc::new(child_ctx);

        // Push child context onto thread-local stack
        push_context(Arc::clone(&child_ctx));

        // Invoke the command
        let result = cmd.invoke(&child_ctx);

        // Pop context
        pop_context();

        // Run close callbacks on child
        child_ctx.close();

        result
    }

    /// Forward the current context's parameters to another command.
    ///
    /// This is similar to [`invoke`](Self::invoke), but reuses the current context's
    /// parameter values instead of parsing new arguments. The invoked command will
    /// see the same parameter values as this context.
    ///
    /// This is useful for delegating to another command while preserving the
    /// current context's state, similar to Python Click's `ctx.forward()`.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command to forward to
    ///
    /// # Example
    ///
    /// ```
    /// use click::command::Command;
    /// use click::context::ContextBuilder;
    /// use click::group::CommandLike;
    /// use std::sync::Arc;
    ///
    /// let other_cmd = Command::new("other")
    ///     .callback(|ctx| {
    ///         // This command will see the forwarded params
    ///         if let Some(name) = ctx.get_param::<String>("name") {
    ///             println!("Forwarded name: {}", name);
    ///         }
    ///         Ok(())
    ///     })
    ///     .build();
    ///
    /// let mut ctx = ContextBuilder::new().info_name("main").build();
    /// ctx.params_mut().insert("name".to_string(), Arc::new("Alice".to_string()));
    /// let ctx = Arc::new(ctx);
    ///
    /// let result = ctx.forward(&other_cmd);
    /// assert!(result.is_ok());
    /// ```
    pub fn forward(
        self: &Arc<Self>,
        cmd: &dyn crate::group::CommandLike,
    ) -> Result<(), ClickError> {
        // Get the command name for the child context
        let cmd_name = cmd.name().unwrap_or("forwarded");

        // Create a child context builder with this context as parent
        let child_builder = ContextBuilder::new()
            .info_name(cmd_name)
            .parent(Arc::clone(self));

        // If the parent has an obj, it will be inherited via ContextBuilder.
        // Build the child context
        let mut child_ctx = child_builder.build();

        // Copy all parameters from this context to the child
        for (key, value) in self.params.iter() {
            child_ctx.params.insert(key.clone(), Arc::clone(value));
        }

        // Copy parameter sources
        for (key, source) in self.parameter_source.iter() {
            child_ctx.parameter_source.insert(key.clone(), *source);
        }

        let child_ctx = Arc::new(child_ctx);

        // Push child context onto thread-local stack
        push_context(Arc::clone(&child_ctx));

        // Invoke the command
        let result = cmd.invoke(&child_ctx);

        // Pop context
        pop_context();

        // Run close callbacks on child
        child_ctx.close();

        result
    }

    /// Execute a function with a resource, ensuring cleanup on close.
    ///
    /// This is a convenience method that combines resource registration with
    /// immediate use. The resource is passed to the provided function, and
    /// a cleanup callback is registered via [`call_on_close`](Self::call_on_close).
    ///
    /// This pattern is useful for managing resources like file handles,
    /// database connections, or other cleanup-requiring objects within
    /// command execution.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The resource type (must be `Send + 'static`)
    /// * `F` - The function to execute with the resource
    /// * `C` - The cleanup function
    /// * `R` - The return type of the function
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to manage
    /// * `f` - The function to execute with the resource
    /// * `cleanup` - The cleanup function to run when the context closes
    ///
    /// # Example
    ///
    /// ```
    /// use click::context::ContextBuilder;
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicBool, Ordering};
    ///
    /// struct Resource {
    ///     value: i32,
    /// }
    ///
    /// let cleaned_up = Arc::new(AtomicBool::new(false));
    /// let cleaned_up_clone = Arc::clone(&cleaned_up);
    ///
    /// let ctx = ContextBuilder::new().build();
    ///
    /// let result = ctx.with_resource(
    ///     Resource { value: 42 },
    ///     |res| res.value * 2,
    ///     move || {
    ///         cleaned_up_clone.store(true, Ordering::SeqCst);
    ///     },
    /// );
    ///
    /// assert_eq!(result, 84);
    /// assert!(!cleaned_up.load(Ordering::SeqCst)); // Not yet cleaned up
    ///
    /// ctx.close();
    /// assert!(cleaned_up.load(Ordering::SeqCst)); // Now cleaned up
    /// ```
    pub fn with_resource<T, F, C, R>(&self, resource: T, f: F, cleanup: C) -> R
    where
        T: Send + 'static,
        F: FnOnce(&T) -> R,
        C: FnOnce() + Send + 'static,
    {
        // Register the cleanup callback
        self.call_on_close(cleanup);

        // Execute the function with the resource
        f(&resource)
    }
}

/// Builder for creating [`Context`] instances with custom settings.
///
/// # Example
///
/// ```
/// use click::context::ContextBuilder;
///
/// let ctx = ContextBuilder::new()
///     .info_name("myapp")
///     .allow_extra_args(true)
///     .terminal_width(100)
///     .color(true)
///     .build();
///
/// assert_eq!(ctx.info_name(), Some("myapp"));
/// assert!(ctx.allow_extra_args());
/// assert_eq!(ctx.terminal_width(), Some(100));
/// assert_eq!(ctx.color(), Some(true));
/// ```
#[derive(Default)]
pub struct ContextBuilder {
    parent: Option<Arc<Context>>,
    info_name: Option<String>,
    obj: Option<BoxedValue>,
    auto_envvar_prefix: Option<String>,
    default_map: Option<HashMap<String, BoxedValue>>,
    terminal_width: Option<usize>,
    max_content_width: Option<usize>,
    resilient_parsing: bool,
    allow_extra_args: Option<bool>,
    allow_interspersed_args: Option<bool>,
    ignore_unknown_options: Option<bool>,
    help_option_names: Option<Vec<String>>,
    color: Option<bool>,
    show_default: Option<bool>,
}

impl ContextBuilder {
    /// Create a new context builder with default settings.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the parent context.
    ///
    /// Settings like `terminal_width`, `color`, and `show_default` will be
    /// inherited from the parent if not explicitly set.
    pub fn parent(mut self, parent: Arc<Context>) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Set the info name for this context.
    ///
    /// This is typically the command name.
    pub fn info_name(mut self, name: impl Into<String>) -> Self {
        self.info_name = Some(name.into());
        self
    }

    /// Set the user object.
    ///
    /// If not set and there's a parent context, the parent's obj will be inherited.
    pub fn obj<T: Any + Send + Sync + 'static>(mut self, obj: T) -> Self {
        self.obj = Some(Arc::new(obj));
        self
    }

    /// Set the auto envvar prefix.
    ///
    /// If set, parameters can automatically read from environment variables
    /// with the format `{PREFIX}_{PARAM_NAME}`.
    pub fn auto_envvar_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.auto_envvar_prefix = Some(prefix.into());
        self
    }

    /// Set the default map.
    ///
    /// This provides default values for parameters that override the parameter's
    /// own default.
    pub fn default_map(mut self, map: HashMap<String, BoxedValue>) -> Self {
        self.default_map = Some(map);
        self
    }

    /// Set the terminal width.
    pub fn terminal_width(mut self, width: usize) -> Self {
        self.terminal_width = Some(width);
        self
    }

    /// Set the maximum content width for help text.
    pub fn max_content_width(mut self, width: usize) -> Self {
        self.max_content_width = Some(width);
        self
    }

    /// Enable or disable resilient parsing mode.
    ///
    /// In resilient parsing mode, Click will parse without interactivity or
    /// callback invocation. This is useful for shell completion.
    pub fn resilient_parsing(mut self, enabled: bool) -> Self {
        self.resilient_parsing = enabled;
        self
    }

    /// Set whether extra arguments are allowed.
    pub fn allow_extra_args(mut self, allow: bool) -> Self {
        self.allow_extra_args = Some(allow);
        self
    }

    /// Set whether interspersed arguments are allowed.
    pub fn allow_interspersed_args(mut self, allow: bool) -> Self {
        self.allow_interspersed_args = Some(allow);
        self
    }

    /// Set whether unknown options should be ignored.
    pub fn ignore_unknown_options(mut self, ignore: bool) -> Self {
        self.ignore_unknown_options = Some(ignore);
        self
    }

    /// Set the help option names.
    ///
    /// Default is `["--help"]`.
    pub fn help_option_names(mut self, names: Vec<String>) -> Self {
        self.help_option_names = Some(names);
        self
    }

    /// Set the color setting.
    pub fn color(mut self, color: bool) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the show_default setting.
    pub fn show_default(mut self, show: bool) -> Self {
        self.show_default = Some(show);
        self
    }

    /// Build the context.
    pub fn build(self) -> Context {
        // Inherit values from parent if not explicitly set
        let (terminal_width, max_content_width, color, show_default, help_option_names, obj, meta) =
            if let Some(ref parent) = self.parent {
                (
                    self.terminal_width.or(parent.terminal_width),
                    self.max_content_width.or(parent.max_content_width),
                    self.color.or(parent.color),
                    self.show_default.or(parent.show_default),
                    self.help_option_names
                        .unwrap_or_else(|| parent.help_option_names.clone()),
                    // Inherit obj from parent if not explicitly set
                    self.obj.or_else(|| parent.obj.clone()),
                    // Clone meta from parent (child starts with parent's metadata)
                    parent.meta.clone(),
                )
            } else {
                (
                    self.terminal_width,
                    self.max_content_width,
                    self.color,
                    self.show_default,
                    self.help_option_names
                        .unwrap_or_else(|| vec!["--help".to_string()]),
                    self.obj,
                    HashMap::new(),
                )
            };

        // Compute auto_envvar_prefix
        let auto_envvar_prefix = if let Some(prefix) = self.auto_envvar_prefix {
            // Normalize: uppercase and replace dashes with underscores
            Some(prefix.to_uppercase().replace('-', "_"))
        } else if let (Some(ref parent), Some(ref info_name)) = (&self.parent, &self.info_name) {
            // Expand from parent's prefix
            parent.auto_envvar_prefix.as_ref().map(|parent_prefix| {
                format!(
                    "{}_{}",
                    parent_prefix,
                    info_name.to_uppercase().replace('-', "_")
                )
            })
        } else {
            None
        };

        // Use provided default_map, or None.
        // Note: Python Click supports nested default_map inheritance from parent,
        // but this requires complex type handling in Rust. For now, each context
        // should have its own default_map if needed.
        let default_map = self.default_map;

        Context {
            parent: self.parent,
            info_name: self.info_name,
            params: HashMap::new(),
            args: Vec::new(),
            obj,
            meta,
            default_map,
            invoked_subcommand: None,
            terminal_width,
            max_content_width,
            allow_extra_args: self.allow_extra_args.unwrap_or(false),
            allow_interspersed_args: self.allow_interspersed_args.unwrap_or(true),
            ignore_unknown_options: self.ignore_unknown_options.unwrap_or(false),
            help_option_names,
            resilient_parsing: self.resilient_parsing,
            auto_envvar_prefix,
            color,
            show_default,
            parameter_source: HashMap::new(),
            close_callbacks: RefCell::new(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_context_default_values() {
        let ctx = Context::new();

        assert!(ctx.parent().is_none());
        assert!(ctx.info_name().is_none());
        assert!(ctx.params().is_empty());
        assert!(ctx.args().is_empty());
        assert!(!ctx.allow_extra_args());
        assert!(ctx.allow_interspersed_args());
        assert!(!ctx.ignore_unknown_options());
        assert_eq!(ctx.help_option_names(), &["--help".to_string()]);
        assert!(!ctx.resilient_parsing());
        assert!(ctx.auto_envvar_prefix().is_none());
        assert!(ctx.color().is_none());
        assert!(ctx.show_default().is_none());
    }

    #[test]
    fn test_context_builder() {
        let ctx = ContextBuilder::new()
            .info_name("myapp")
            .allow_extra_args(true)
            .allow_interspersed_args(false)
            .ignore_unknown_options(true)
            .terminal_width(120)
            .max_content_width(100)
            .resilient_parsing(true)
            .auto_envvar_prefix("MYAPP")
            .color(true)
            .show_default(false)
            .help_option_names(vec!["--help".to_string(), "-h".to_string()])
            .build();

        assert_eq!(ctx.info_name(), Some("myapp"));
        assert!(ctx.allow_extra_args());
        assert!(!ctx.allow_interspersed_args());
        assert!(ctx.ignore_unknown_options());
        assert_eq!(ctx.terminal_width(), Some(120));
        assert_eq!(ctx.max_content_width(), Some(100));
        assert!(ctx.resilient_parsing());
        assert_eq!(ctx.auto_envvar_prefix(), Some("MYAPP"));
        assert_eq!(ctx.color(), Some(true));
        assert_eq!(ctx.show_default(), Some(false));
        assert_eq!(
            ctx.help_option_names(),
            &["--help".to_string(), "-h".to_string()]
        );
    }

    #[test]
    fn test_parent_child_context_chain() {
        let parent = Arc::new(
            ContextBuilder::new()
                .info_name("cli")
                .terminal_width(100)
                .color(true)
                .build(),
        );

        let child = ContextBuilder::new()
            .info_name("subcommand")
            .parent(Arc::clone(&parent))
            .build();

        // Child inherits from parent
        assert_eq!(child.terminal_width(), Some(100));
        assert_eq!(child.color(), Some(true));

        // Parent reference is correct
        assert!(child.parent().is_some());
        assert_eq!(child.parent().unwrap().info_name(), Some("cli"));
    }

    #[test]
    fn test_command_path() {
        let root = Arc::new(ContextBuilder::new().info_name("cli").build());
        let sub = Arc::new(
            ContextBuilder::new()
                .info_name("group")
                .parent(Arc::clone(&root))
                .build(),
        );
        let leaf = ContextBuilder::new()
            .info_name("command")
            .parent(sub)
            .build();

        assert_eq!(root.command_path(), "cli");
        assert_eq!(leaf.command_path(), "cli group command");
    }

    #[test]
    fn test_find_root() {
        let root = Arc::new(ContextBuilder::new().info_name("cli").build());
        let child = Arc::new(
            ContextBuilder::new()
                .info_name("sub")
                .parent(Arc::clone(&root))
                .build(),
        );
        let grandchild = ContextBuilder::new()
            .info_name("leaf")
            .parent(child)
            .build();

        assert_eq!(grandchild.find_root().info_name(), Some("cli"));
    }

    #[test]
    fn test_thread_local_stack() {
        // Ensure stack is empty initially
        assert!(get_current_context().is_none());

        let ctx1 = Arc::new(ContextBuilder::new().info_name("ctx1").build());
        let ctx2 = Arc::new(ContextBuilder::new().info_name("ctx2").build());

        // Push first context
        push_context(Arc::clone(&ctx1));
        assert_eq!(get_current_context().unwrap().info_name(), Some("ctx1"));

        // Push second context
        push_context(Arc::clone(&ctx2));
        assert_eq!(get_current_context().unwrap().info_name(), Some("ctx2"));

        // Pop returns the top context
        let popped = pop_context();
        assert_eq!(popped.unwrap().info_name(), Some("ctx2"));
        assert_eq!(get_current_context().unwrap().info_name(), Some("ctx1"));

        // Clean up
        pop_context();
        assert!(get_current_context().is_none());
    }

    #[test]
    fn test_parameter_source_tracking() {
        let mut ctx = Context::new();

        // Initially no source
        assert!(ctx.get_parameter_source("name").is_none());

        // Set source
        ctx.set_parameter_source("name", ParameterSource::CommandLine);
        assert_eq!(
            ctx.get_parameter_source("name"),
            Some(ParameterSource::CommandLine)
        );

        // Update source
        ctx.set_parameter_source("name", ParameterSource::Environment);
        assert_eq!(
            ctx.get_parameter_source("name"),
            Some(ParameterSource::Environment)
        );
    }

    #[test]
    fn test_close_callbacks() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = Arc::clone(&counter);
        let c2 = Arc::clone(&counter);
        let c3 = Arc::clone(&counter);

        let ctx = Context::new();

        // Register callbacks - they record the order they were called
        ctx.call_on_close(move || {
            c1.fetch_add(1, Ordering::SeqCst);
        });
        ctx.call_on_close(move || {
            c2.fetch_add(10, Ordering::SeqCst);
        });
        ctx.call_on_close(move || {
            c3.fetch_add(100, Ordering::SeqCst);
        });

        assert_eq!(counter.load(Ordering::SeqCst), 0);

        ctx.close();

        // All callbacks should have run
        assert_eq!(counter.load(Ordering::SeqCst), 111);

        // Calling close again should do nothing (callbacks are drained)
        ctx.close();
        assert_eq!(counter.load(Ordering::SeqCst), 111);
    }

    #[test]
    fn test_error_creation() {
        let ctx = ContextBuilder::new().info_name("myapp").build();

        let usage_err = ctx.fail("something went wrong");
        assert!(matches!(usage_err, ClickError::UsageError { .. }));
        assert_eq!(usage_err.exit_code(), 2);

        let abort_err = ctx.abort();
        assert!(matches!(abort_err, ClickError::Abort));
        assert_eq!(abort_err.exit_code(), 1);

        let exit_err = ctx.exit(42);
        assert!(matches!(exit_err, ClickError::Exit { code: 42 }));
        assert_eq!(exit_err.exit_code(), 42);
    }

    #[test]
    fn test_params_access() {
        let mut ctx = Context::new();

        ctx.params_mut()
            .insert("count".to_string(), Arc::new(42i32));
        ctx.params_mut()
            .insert("name".to_string(), Arc::new("Alice".to_string()));

        assert_eq!(ctx.get_param::<i32>("count"), Some(&42));
        assert_eq!(ctx.get_param::<String>("name"), Some(&"Alice".to_string()));
        assert!(ctx.get_param::<i32>("name").is_none()); // Wrong type
        assert!(ctx.get_param::<i32>("missing").is_none()); // Missing
    }

    #[test]
    fn test_obj_access() {
        #[derive(Debug, PartialEq)]
        struct AppState {
            value: i32,
        }

        let ctx = ContextBuilder::new().obj(AppState { value: 123 }).build();

        let state = ctx.obj::<AppState>().unwrap();
        assert_eq!(state.value, 123);

        // Wrong type returns None
        assert!(ctx.obj::<String>().is_none());
    }

    #[test]
    fn test_lookup_default() {
        let mut defaults: HashMap<String, BoxedValue> = HashMap::new();
        defaults.insert("count".to_string(), Arc::new(42i32));
        defaults.insert("name".to_string(), Arc::new("default".to_string()));

        let ctx = ContextBuilder::new().default_map(defaults).build();

        let count_default = ctx.lookup_default("count").unwrap();
        assert_eq!(count_default.downcast_ref::<i32>(), Some(&42));

        let name_default = ctx.lookup_default("name").unwrap();
        assert_eq!(
            name_default.downcast_ref::<String>(),
            Some(&"default".to_string())
        );

        assert!(ctx.lookup_default("missing").is_none());
    }

    #[test]
    fn test_auto_envvar_prefix_normalization() {
        // Direct prefix is normalized
        let ctx = ContextBuilder::new().auto_envvar_prefix("my-app").build();
        assert_eq!(ctx.auto_envvar_prefix(), Some("MY_APP"));

        // Inherited prefix is expanded
        let parent = Arc::new(ContextBuilder::new().auto_envvar_prefix("MY_APP").build());
        let child = ContextBuilder::new()
            .info_name("sub-cmd")
            .parent(parent)
            .build();
        assert_eq!(child.auto_envvar_prefix(), Some("MY_APP_SUB_CMD"));
    }

    #[test]
    fn test_args_access() {
        let mut ctx = Context::new();

        assert!(ctx.args().is_empty());

        ctx.args_mut().push("extra1".to_string());
        ctx.args_mut().push("extra2".to_string());

        assert_eq!(ctx.args(), &["extra1", "extra2"]);
    }

    #[test]
    fn test_meta_access() {
        let mut ctx = Context::new();

        ctx.meta_mut()
            .insert("mymodule.key".to_string(), Arc::new(42i32));

        assert_eq!(ctx.get_meta::<i32>("mymodule.key"), Some(&42));
        assert!(ctx.get_meta::<String>("mymodule.key").is_none()); // Wrong type
        assert!(ctx.get_meta::<i32>("other.key").is_none()); // Missing
    }

    #[test]
    fn test_invoked_subcommand() {
        let mut ctx = Context::new();

        assert!(ctx.invoked_subcommand().is_none());

        ctx.set_invoked_subcommand(Some("subcommand".to_string()));
        assert_eq!(ctx.invoked_subcommand(), Some("subcommand"));

        ctx.set_invoked_subcommand(Some("*".to_string()));
        assert_eq!(ctx.invoked_subcommand(), Some("*"));

        ctx.set_invoked_subcommand(None);
        assert!(ctx.invoked_subcommand().is_none());
    }

    // =========================================================================
    // Tests for Context helper methods: invoke, forward, with_resource
    // =========================================================================

    #[test]
    fn test_context_invoke_command() {
        use crate::command::Command;
        use std::sync::atomic::{AtomicBool, Ordering};

        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = Arc::clone(&invoked);

        let other_cmd = Command::new("other")
            .callback(move |_ctx| {
                invoked_clone.store(true, Ordering::SeqCst);
                Ok(())
            })
            .build();

        let ctx = Arc::new(ContextBuilder::new().info_name("main").build());
        let result = ctx.invoke(&other_cmd, &[]);

        assert!(result.is_ok());
        assert!(invoked.load(Ordering::SeqCst));
    }

    #[test]
    fn test_context_invoke_with_args() {
        use crate::command::Command;
        use crate::argument::Argument;
        use std::sync::Mutex;

        let captured_name = Arc::new(Mutex::new(String::new()));
        let captured_clone = Arc::clone(&captured_name);

        let other_cmd = Command::new("greet")
            .argument(Argument::new("name").build())
            .callback(move |ctx| {
                if let Some(name) = ctx.get_param::<String>("name") {
                    let mut lock = captured_clone.lock().unwrap();
                    *lock = name.clone();
                }
                Ok(())
            })
            .build();

        let ctx = Arc::new(ContextBuilder::new().info_name("main").build());
        let result = ctx.invoke(&other_cmd, &["Alice".to_string()]);

        assert!(result.is_ok());
        let name = captured_name.lock().unwrap();
        assert_eq!(*name, "Alice");
    }

    #[test]
    fn test_context_invoke_creates_child_context() {
        use crate::command::Command;
        use std::sync::Mutex;

        let parent_name = Arc::new(Mutex::new(None::<String>));
        let parent_clone = Arc::clone(&parent_name);

        let other_cmd = Command::new("child")
            .callback(move |ctx| {
                if let Some(parent) = ctx.parent() {
                    let mut lock = parent_clone.lock().unwrap();
                    *lock = parent.info_name().map(|s| s.to_string());
                }
                Ok(())
            })
            .build();

        let ctx = Arc::new(ContextBuilder::new().info_name("main").build());
        let result = ctx.invoke(&other_cmd, &[]);

        assert!(result.is_ok());
        let captured = parent_name.lock().unwrap();
        assert_eq!(*captured, Some("main".to_string()));
    }

    #[test]
    fn test_context_forward_copies_params() {
        use crate::command::Command;
        use std::sync::Mutex;

        let forwarded_name = Arc::new(Mutex::new(None::<String>));
        let forwarded_clone = Arc::clone(&forwarded_name);

        let other_cmd = Command::new("receiver")
            .callback(move |ctx| {
                let mut lock = forwarded_clone.lock().unwrap();
                *lock = ctx.get_param::<String>("name").cloned();
                Ok(())
            })
            .build();

        let mut ctx = ContextBuilder::new().info_name("sender").build();
        ctx.params_mut().insert("name".to_string(), Arc::new("Forwarded".to_string()));
        let ctx = Arc::new(ctx);

        let result = ctx.forward(&other_cmd);

        assert!(result.is_ok());
        let captured = forwarded_name.lock().unwrap();
        assert_eq!(*captured, Some("Forwarded".to_string()));
    }

    #[test]
    fn test_context_forward_copies_multiple_params() {
        use crate::command::Command;
        use std::sync::Mutex;

        let forwarded_params = Arc::new(Mutex::new((None::<String>, None::<i32>)));
        let params_clone = Arc::clone(&forwarded_params);

        let other_cmd = Command::new("receiver")
            .callback(move |ctx| {
                let mut lock = params_clone.lock().unwrap();
                lock.0 = ctx.get_param::<String>("name").cloned();
                lock.1 = ctx.get_param::<i32>("count").copied();
                Ok(())
            })
            .build();

        let mut ctx = ContextBuilder::new().info_name("sender").build();
        ctx.params_mut().insert("name".to_string(), Arc::new("Test".to_string()));
        ctx.params_mut().insert("count".to_string(), Arc::new(42i32));
        let ctx = Arc::new(ctx);

        let result = ctx.forward(&other_cmd);

        assert!(result.is_ok());
        let captured = forwarded_params.lock().unwrap();
        assert_eq!(captured.0, Some("Test".to_string()));
        assert_eq!(captured.1, Some(42));
    }

    #[test]
    fn test_context_forward_copies_parameter_sources() {
        use crate::command::Command;
        use std::sync::Mutex;

        let forwarded_source = Arc::new(Mutex::new(None::<ParameterSource>));
        let source_clone = Arc::clone(&forwarded_source);

        let other_cmd = Command::new("receiver")
            .callback(move |ctx| {
                let mut lock = source_clone.lock().unwrap();
                *lock = ctx.get_parameter_source("name");
                Ok(())
            })
            .build();

        let mut ctx = ContextBuilder::new().info_name("sender").build();
        ctx.params_mut().insert("name".to_string(), Arc::new("Test".to_string()));
        ctx.set_parameter_source("name", ParameterSource::CommandLine);
        let ctx = Arc::new(ctx);

        let result = ctx.forward(&other_cmd);

        assert!(result.is_ok());
        let captured = forwarded_source.lock().unwrap();
        assert_eq!(*captured, Some(ParameterSource::CommandLine));
    }

    #[test]
    fn test_context_forward_creates_child_context() {
        use crate::command::Command;
        use std::sync::Mutex;

        let parent_name = Arc::new(Mutex::new(None::<String>));
        let parent_clone = Arc::clone(&parent_name);

        let other_cmd = Command::new("receiver")
            .callback(move |ctx| {
                if let Some(parent) = ctx.parent() {
                    let mut lock = parent_clone.lock().unwrap();
                    *lock = parent.info_name().map(|s| s.to_string());
                }
                Ok(())
            })
            .build();

        let ctx = Arc::new(ContextBuilder::new().info_name("sender").build());
        let result = ctx.forward(&other_cmd);

        assert!(result.is_ok());
        let captured = parent_name.lock().unwrap();
        assert_eq!(*captured, Some("sender".to_string()));
    }

    #[test]
    fn test_with_resource_basic() {
        use std::sync::atomic::{AtomicBool, Ordering};

        struct Resource {
            value: i32,
        }

        let cleaned_up = Arc::new(AtomicBool::new(false));
        let cleaned_up_clone = Arc::clone(&cleaned_up);

        let ctx = ContextBuilder::new().build();

        let result = ctx.with_resource(
            Resource { value: 42 },
            |res| res.value * 2,
            move || {
                cleaned_up_clone.store(true, Ordering::SeqCst);
            },
        );

        assert_eq!(result, 84);
        assert!(!cleaned_up.load(Ordering::SeqCst)); // Not yet cleaned up

        ctx.close();
        assert!(cleaned_up.load(Ordering::SeqCst)); // Now cleaned up
    }

    #[test]
    fn test_with_resource_multiple() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let cleanup_count = Arc::new(AtomicUsize::new(0));
        let count1 = Arc::clone(&cleanup_count);
        let count2 = Arc::clone(&cleanup_count);

        let ctx = ContextBuilder::new().build();

        let result1 = ctx.with_resource(
            10,
            |res| *res + 5,
            move || {
                count1.fetch_add(1, Ordering::SeqCst);
            },
        );

        let result2 = ctx.with_resource(
            20,
            |res| *res * 2,
            move || {
                count2.fetch_add(1, Ordering::SeqCst);
            },
        );

        assert_eq!(result1, 15);
        assert_eq!(result2, 40);
        assert_eq!(cleanup_count.load(Ordering::SeqCst), 0);

        ctx.close();
        assert_eq!(cleanup_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_with_resource_string_resource() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let cleaned_up = Arc::new(AtomicBool::new(false));
        let cleaned_up_clone = Arc::clone(&cleaned_up);

        let ctx = ContextBuilder::new().build();

        let result = ctx.with_resource(
            String::from("hello"),
            |s| s.len(),
            move || {
                cleaned_up_clone.store(true, Ordering::SeqCst);
            },
        );

        assert_eq!(result, 5);

        ctx.close();
        assert!(cleaned_up.load(Ordering::SeqCst));
    }

    #[test]
    fn test_invoke_error_propagation() {
        use crate::command::Command;
        use crate::error::ClickError;

        let other_cmd = Command::new("failing")
            .callback(|_ctx| {
                Err(ClickError::usage("intentional failure"))
            })
            .build();

        let ctx = Arc::new(ContextBuilder::new().info_name("main").build());
        let result = ctx.invoke(&other_cmd, &[]);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ClickError::UsageError { .. }));
    }

    #[test]
    fn test_forward_error_propagation() {
        use crate::command::Command;
        use crate::error::ClickError;

        let other_cmd = Command::new("failing")
            .callback(|_ctx| {
                Err(ClickError::usage("intentional failure"))
            })
            .build();

        let ctx = Arc::new(ContextBuilder::new().info_name("main").build());
        let result = ctx.forward(&other_cmd);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ClickError::UsageError { .. }));
    }

    #[test]
    fn test_invoke_runs_child_close_callbacks() {
        use crate::command::Command;
        use std::sync::atomic::{AtomicBool, Ordering};

        let child_closed = Arc::new(AtomicBool::new(false));
        let closed_clone = Arc::clone(&child_closed);

        let other_cmd = Command::new("child")
            .callback(move |ctx| {
                let closed_clone = Arc::clone(&closed_clone);
                ctx.call_on_close(move || {
                    closed_clone.store(true, Ordering::SeqCst);
                });
                Ok(())
            })
            .build();

        let ctx = Arc::new(ContextBuilder::new().info_name("main").build());
        let result = ctx.invoke(&other_cmd, &[]);

        assert!(result.is_ok());
        // Child context should have been closed after invoke
        assert!(child_closed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_forward_runs_child_close_callbacks() {
        use crate::command::Command;
        use std::sync::atomic::{AtomicBool, Ordering};

        let child_closed = Arc::new(AtomicBool::new(false));
        let closed_clone = Arc::clone(&child_closed);

        let other_cmd = Command::new("child")
            .callback(move |ctx| {
                let closed_clone = Arc::clone(&closed_clone);
                ctx.call_on_close(move || {
                    closed_clone.store(true, Ordering::SeqCst);
                });
                Ok(())
            })
            .build();

        let ctx = Arc::new(ContextBuilder::new().info_name("main").build());
        let result = ctx.forward(&other_cmd);

        assert!(result.is_ok());
        // Child context should have been closed after forward
        assert!(child_closed.load(Ordering::SeqCst));
    }
}
