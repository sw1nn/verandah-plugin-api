// Allow non_local_definitions warning from abi_stable's sabi_trait macro
// This is a known issue with the macro-generated code
#![allow(non_local_definitions)]
//! FFI-safe plugin API for verandah live widgets
//!
//! This crate defines the ABI-stable interface that plugins must implement
//! to provide live widget functionality.
//!
//! ## Logging
//!
//! Plugins can use standard `tracing` macros (`tracing::info!`, `tracing::debug!`, etc.)
//! and logs will automatically be forwarded to the host daemon's logging system.
//! The host calls `set_logger_impl` after loading the plugin to set up the bridge.

mod config;
pub use config::parse_config;
pub use toml;

use std::sync::atomic::{AtomicPtr, Ordering};

use abi_stable::{
    StableAbi, declare_root_module_statics,
    library::RootModule,
    package_version_strings, sabi_trait,
    std_types::{RBox, RHashMap, RResult, RStr, RString, RVec},
};
use tracing_subscriber::layer::SubscriberExt;

/// FFI-safe log entry with structured fields
#[repr(C)]
#[derive(StableAbi, Clone, Debug)]
pub struct PluginLogEntry {
    pub level: PluginLogLevel,
    pub target: RString,
    pub message: RString,
    /// Structured key-value fields
    pub fields: RHashMap<RString, RString>,
}

/// Log level for plugin messages
#[repr(C)]
#[derive(StableAbi, Clone, Copy, Debug)]
pub enum PluginLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Global storage for the logging callback
static LOG_CALLBACK: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

/// A tracing Layer that forwards events to the host's logging callback
struct PluginLogLayer;

impl<S> tracing_subscriber::Layer<S> for PluginLogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let ptr = LOG_CALLBACK.load(Ordering::SeqCst);
        if ptr.is_null() {
            return;
        }

        let callback: extern "C" fn(&PluginLogEntry) = unsafe { std::mem::transmute(ptr) };

        // Convert tracing level to PluginLogLevel
        let level = match *event.metadata().level() {
            tracing::Level::ERROR => PluginLogLevel::Error,
            tracing::Level::WARN => PluginLogLevel::Warn,
            tracing::Level::INFO => PluginLogLevel::Info,
            tracing::Level::DEBUG => PluginLogLevel::Debug,
            tracing::Level::TRACE => PluginLogLevel::Trace,
        };

        // Extract message and fields using a visitor
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let entry = PluginLogEntry {
            level,
            target: event.metadata().target().into(),
            message: visitor.message.into(),
            fields: visitor.fields,
        };

        callback(&entry);
    }
}

/// Visitor to extract fields from tracing events
#[derive(Default)]
struct FieldVisitor {
    message: String,
    fields: RHashMap<RString, RString>,
}

impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields
                .insert(field.name().into(), format!("{value:?}").into());
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.insert(field.name().into(), value.into());
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().into(), value.to_string().into());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().into(), value.to_string().into());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().into(), value.to_string().into());
    }
}

/// Set the logging callback and install the tracing subscriber
///
/// This is called by the host after loading the plugin. Plugins should not call
/// this directly. After this is called, standard `tracing` macros will forward
/// logs to the host's logging system.
pub extern "C" fn set_logger_impl(callback: *mut ()) {
    LOG_CALLBACK.store(callback, Ordering::SeqCst);

    // Set up tracing subscriber with our layer
    // Use try_init to avoid panic if already set (e.g., in tests)
    let subscriber = tracing_subscriber::registry().with(PluginLogLayer);
    let _ = tracing::subscriber::set_global_default(subscriber);
}

/// FFI-safe widget state enum
#[repr(C)]
#[derive(StableAbi, Clone, Debug, PartialEq)]
pub enum PluginWidgetState {
    Text(RString),
    Boolean(bool),
    Numeric(f64),
}

/// Response from poll_state containing state and optional interval hint
#[repr(C)]
#[derive(StableAbi, Clone, Debug)]
pub struct PluginPollResponse {
    /// The current widget state
    pub state: PluginWidgetState,
    /// Optional interval hint for next poll (None = use default interval)
    pub interval_hint: abi_stable::std_types::ROption<PluginDuration>,
}

impl PluginPollResponse {
    /// Create a response with just state (uses default interval)
    pub fn state(state: PluginWidgetState) -> Self {
        PluginPollResponse {
            state,
            interval_hint: abi_stable::std_types::ROption::RNone,
        }
    }

    /// Create a response with state and interval hint
    pub fn with_interval(state: PluginWidgetState, interval: PluginDuration) -> Self {
        PluginPollResponse {
            state,
            interval_hint: abi_stable::std_types::ROption::RSome(interval),
        }
    }
}

/// FFI-safe duration representation
#[repr(C)]
#[derive(StableAbi, Clone, Copy, Debug)]
pub struct PluginDuration {
    pub millis: u64,
}

impl PluginDuration {
    pub const fn from_millis(millis: u64) -> Self {
        PluginDuration { millis }
    }

    pub const fn from_secs(secs: u64) -> Self {
        PluginDuration {
            millis: secs * 1000,
        }
    }
}

impl From<std::time::Duration> for PluginDuration {
    fn from(d: std::time::Duration) -> Self {
        PluginDuration {
            millis: d.as_millis() as u64,
        }
    }
}

impl From<PluginDuration> for std::time::Duration {
    fn from(d: PluginDuration) -> Self {
        std::time::Duration::from_millis(d.millis)
    }
}

/// FFI-safe image size
#[repr(C)]
#[derive(StableAbi, Clone, Copy, Debug)]
pub struct PluginImageSize {
    pub width: u32,
    pub height: u32,
}

impl PluginImageSize {
    pub const fn new(width: u32, height: u32) -> Self {
        PluginImageSize { width, height }
    }
}

impl From<(u32, u32)> for PluginImageSize {
    fn from(size: (u32, u32)) -> Self {
        PluginImageSize {
            width: size.0,
            height: size.1,
        }
    }
}

impl From<PluginImageSize> for (u32, u32) {
    fn from(size: PluginImageSize) -> Self {
        (size.width, size.height)
    }
}

/// FFI-safe image data (RGB8 format)
#[repr(C)]
#[derive(StableAbi, Clone, Debug)]
pub struct PluginImage {
    pub width: u32,
    pub height: u32,
    pub data: RVec<u8>,
}

impl PluginImage {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 3) as usize;
        PluginImage {
            width,
            height,
            data: RVec::from(vec![0u8; size]),
        }
    }

    pub fn from_rgb(width: u32, height: u32, data: Vec<u8>) -> Self {
        PluginImage {
            width,
            height,
            data: RVec::from(data),
        }
    }
}

/// FFI-safe error type for plugins
#[repr(C)]
#[derive(StableAbi, Clone, Debug)]
pub struct PluginError {
    pub message: RString,
}

impl PluginError {
    pub fn new<S>(message: S) -> Self
    where
        S: Into<RString>,
    {
        PluginError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PluginError {}

/// A named action a plugin can perform in response to a button gesture.
///
/// Declared via [`WidgetPlugin::supported_actions`]; the host validates
/// configured verbs against these and dispatches them to
/// [`WidgetPlugin::handle_action`].
#[repr(C)]
#[derive(StableAbi, Clone, Debug)]
pub struct PluginActionSpec {
    /// Verb name used in deck config, e.g. `"toggle"`
    pub name: RString,
    /// Human-readable description for logs and future UI surfaces
    pub description: RString,
    /// At most one spec per plugin may be the default; it fires on a short
    /// press when the key has no configured action.
    pub is_default: bool,
}

impl PluginActionSpec {
    pub fn new<N, D>(name: N, description: D) -> Self
    where
        N: Into<RString>,
        D: Into<RString>,
    {
        PluginActionSpec {
            name: name.into(),
            description: description.into(),
            is_default: false,
        }
    }

    pub fn with_default(mut self) -> Self {
        self.is_default = true;
        self
    }
}

/// Result type for plugin operations
pub type PluginResult<T> = RResult<T, PluginError>;

/// Configuration passed to plugins as serialized TOML
///
/// Plugins can deserialize this using `toml::from_str(&config)` to get
/// their typed configuration struct directly. This allows for nested
/// configuration values like `colors = {fg = "#fff", bg = "#000"}`.
pub type PluginConfig = RString;

/// The main widget plugin trait
///
/// Plugins implement this trait to provide live widget functionality.
#[sabi_trait]
pub trait WidgetPlugin: Send {
    /// Returns the widget type identifier (e.g., "clock", "pipewire-mute")
    fn widget_type(&self) -> RStr<'_>;

    /// Returns the default polling interval
    fn default_interval(&self) -> PluginDuration;

    /// Initialize the widget with configuration
    fn init(&mut self, config: PluginConfig) -> PluginResult<()>;

    /// Returns the current configuration
    fn config(&self) -> PluginConfig;

    /// Poll for current state and optional interval hint
    ///
    /// Returns the current state and optionally a hint for when to poll next.
    /// If no interval hint is provided, the default interval is used.
    fn poll_state(&mut self) -> PluginResult<PluginPollResponse>;

    /// Render the widget to an image
    ///
    /// # Arguments
    /// * `images` - Named images loaded from config (e.g., "muted", "unmuted")
    /// * `state` - Current widget state to render
    /// * `image_size` - Target image dimensions
    fn render(
        &self,
        images: RHashMap<RString, PluginImage>,
        state: &PluginWidgetState,
        image_size: PluginImageSize,
    ) -> PluginResult<PluginImage>;

    /// Clean up resources before the widget is dropped
    ///
    /// Called by the host when the widget is being unregistered or the daemon
    /// is shutting down. Plugins should use this to stop background threads,
    /// close connections, and release any resources.
    fn shutdown(&mut self);

    /// Verbs this plugin can perform. Empty (the common case for passive
    /// widgets) means the plugin has no actions and never receives
    /// `handle_action` calls.
    fn supported_actions(&self) -> RVec<PluginActionSpec>;

    /// Perform the behaviour bound to `verb`. Called on the host's widget
    /// poll thread; after `ROk` the host immediately re-polls and re-renders.
    fn handle_action(&mut self, verb: RStr<'_>) -> PluginResult<()>;
}

/// The root module that plugins must export
///
/// This struct defines the entry point for plugin loading.
#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = PluginModRef)))]
#[sabi(missing_field(panic))]
pub struct PluginMod {
    /// Creates a new widget instance
    pub new: extern "C" fn() -> WidgetPlugin_TO<'static, RBox<()>>,

    /// Plugin API version for compatibility checking
    pub plugin_api_version: RStr<'static>,

    /// Set the logging callback (called by host after loading)
    /// Takes a raw pointer for ABI stability (nested fn pointers don't impl StableAbi)
    #[sabi(last_prefix_field)]
    pub set_logger: extern "C" fn(*mut ()),
}

impl RootModule for PluginModRef {
    declare_root_module_statics! {PluginModRef}
    const BASE_NAME: &'static str = "verandah_plugin";
    const NAME: &'static str = "verandah_plugin";
    const VERSION_STRINGS: abi_stable::sabi_types::VersionStrings = package_version_strings!();
}

/// Current plugin API version (derived from crate version)
pub const PLUGIN_API_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Prelude module for convenient imports
///
/// This module re-exports all commonly used types and functions for plugin development.
/// Plugin authors can use `use verandah_plugin_api::prelude::*;` to import everything needed.
pub mod prelude {
    pub use abi_stable::{
        export_root_module,
        prefix_type::PrefixTypeTrait,
        sabi_extern_fn,
        std_types::{RBox, RHashMap, RString, RStr, RVec},
    };
    pub use serde::Deserialize;

    pub use crate::{
        PLUGIN_API_VERSION, PluginActionSpec, PluginConfig, PluginDuration, PluginError,
        PluginImage, PluginImageSize, PluginMod, PluginModRef, PluginPollResponse,
        PluginResult, PluginWidgetState, WidgetPlugin, WidgetPlugin_TO, parse_config,
        set_logger_impl,
    };
}
