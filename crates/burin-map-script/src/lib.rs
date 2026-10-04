//! burin-map documents in JavaScript.
//!
//! A document is a script that calls our library (`library.js`, over the
//! natives below); running it builds the list of batches and water passes the
//! engine draws, and hands it back as atlas JSON. The script never touches
//! geometry: by the time anything is drawn, the interpreter is gone and only
//! the atlas is left.
//!
//! ```js
//! burin
//!   .atlas()
//!   .batch(burin.batch({ shade: burin.lerp("#e8dcc8", "#2a1810", burin.tone) }))
//!   .water(burin.water("dots"))
//!   .draw();
//! ```
//!
//! This is the same document format draw.land reads through `burin.terrain`,
//! so a style written for one page draws the same in the other.
//!
//! Every error (a syntax error, a wrong argument, a bad atlas) comes back as
//! `line N column M: …`, the form the web editor underlines.
//!
//! The sandbox is the bare language plus the library: no network, files,
//! timers or console, no `Date` or `Math.random` (the same script always draws
//! the same map), and loops and recursion are capped.

use boa_engine::{js_string, Context, JsError, JsNativeError, JsResult, JsValue, NativeFunction, Source};
use serde_json::Value;
use std::fmt;

#[cfg(test)]
mod tests;

const LIBRARY: &str = include_str!("library.js");

/// The name scripts run under; positions in other sources (the library) are
/// not the user's.
pub const SCRIPT_PATH: &str = "document.js";
const LIBRARY_PATH: &str = "burin:library";

/// The most loop iterations one loop may run, and the deepest recursion: a
/// mistaken `while (true)` ends with an error instead of hanging the page.
pub const LOOP_LIMIT: u64 = 1_000_000;
pub const RECURSION_LIMIT: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct At(pub u32, pub u32);

impl fmt::Display for At {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "line {} column {}", self.0, self.1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    pub message: String,
    pub at: Option<At>,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.at {
            Some(at) => write!(f, "{at}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for ScriptError {}

/// Run `source` and return the atlas it built, as JSON.
pub fn evaluate(source: &str) -> Result<Value, ScriptError> {
    SAVED.with(|s| *s.borrow_mut() = None);
    FAILURE.with(|f| *f.borrow_mut() = None);
    let mut ctx = Context::default();
    let limits = ctx.runtime_limits_mut();
    limits.set_loop_iteration_limit(LOOP_LIMIT);
    limits.set_recursion_limit(RECURSION_LIMIT);
    install(&mut ctx).map_err(|e| ScriptError { message: format!("library: {e}"), at: None })?;
    let script = Source::from_bytes(source).with_path(std::path::Path::new(SCRIPT_PATH));
    ctx.eval(script).map_err(|e| script_error(e, source, &mut ctx))?;
    SAVED.with(|s| s.borrow_mut().take()).ok_or_else(|| ScriptError {
        message: "the script never called .draw() or .json() on an atlas".into(),
        at: None,
    })
}

thread_local! {
    /// The atlas JSON the script last handed over.
    static SAVED: std::cell::RefCell<Option<Value>> = const { std::cell::RefCell::new(None) };
}

/// The sandbox, the natives and the library.
fn install(ctx: &mut Context) -> JsResult<()> {
    ctx.eval(Source::from_bytes(
        "delete globalThis.Date; delete Math.random; delete globalThis.WeakRef; delete globalThis.FinalizationRegistry;",
    ))?;
    let natives: [(&str, usize, fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue>); 6] = [
        ("batch", 1, native_batch),
        ("water", 1, native_water),
        ("shore", 1, native_shore),
        ("parameters", 0, native_parameters),
        ("atlas", 1, native_atlas),
        ("draw", 1, native_draw),
    ];
    let b = boa_engine::object::ObjectInitializer::new(ctx).build();
    for (name, arity, f) in natives {
        let function = boa_engine::object::FunctionObjectBuilder::new(ctx.realm(), NativeFunction::from_fn_ptr(f))
            .name(js_string!(name))
            .length(arity)
            .build();
        b.set(js_string!(name), function, false, ctx)?;
    }
    ctx.register_global_property(js_string!("__burin"), b, boa_engine::property::Attribute::all())?;
    ctx.eval(Source::from_bytes(LIBRARY).with_path(std::path::Path::new(LIBRARY_PATH)))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// The natives
// ---------------------------------------------------------------------------

/// The position of the user's script in the current call frame.
fn user_at(ctx: &mut Context) -> Option<At> {
    let error = ctx.intrinsics().constructors().error().constructor();
    let error = error.construct(&[], None, ctx).ok()?;
    let stack = error.get(js_string!("stack"), ctx).ok()?.to_string(ctx).ok()?.to_std_string_escaped();
    let marker = format!("({SCRIPT_PATH}:");
    stack.lines().find_map(|line| {
        let spot = &line[line.find(&marker)? + marker.len()..];
        let (l, c) = spot.trim_end_matches(')').split_once(':')?;
        Some(At(l.parse().ok()?, c.parse().ok()?))
    })
}

/// A native's mistake, positioned at the call in the user's script.
fn fail(ctx: &mut Context, message: impl Into<String>) -> JsError {
    let message = message.into();
    let at = user_at(ctx);
    FAILURE.with(|f| *f.borrow_mut() = Some((message.clone(), at)));
    JsNativeError::typ().with_message(message).into()
}

thread_local! {
    /// A native's message and position, so `evaluate` can report them even
    /// when Boa's own error text does not carry the position.
    static FAILURE: std::cell::RefCell<Option<(String, Option<At>)>> = const { std::cell::RefCell::new(None) };
}

fn json_arg(args: &[JsValue], i: usize, ctx: &mut Context) -> JsResult<Value> {
    let value = args.get(i).cloned().unwrap_or_default();
    // The library hands a JSON string; a bare object is accepted too, for a
    // caller that builds one by hand.
    if let Some(text) = value.as_string() {
        let text = text.to_std_string_escaped();
        return serde_json::from_str(&text)
            .map_err(|e| JsNativeError::typ().with_message(format!("invalid JSON: {e}")).into());
    }
    Ok(value.to_json(ctx)?.unwrap_or(Value::Null))
}

fn to_js(v: &Value, ctx: &mut Context) -> JsResult<JsValue> {
    JsValue::from_json(v, ctx)
}

/// `B.batch(json)`: checked, then kept as one entry.
fn native_batch(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let v = json_arg(args, 0, ctx)?;
    burin_map_core::script::batch_from(&v).map_err(|e| fail(ctx, e))?;
    to_js(&v, ctx)
}

/// `B.water(json)`: a water pass, checked.
fn native_water(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let v = json_arg(args, 0, ctx)?;
    let kind = v.get("draw").and_then(Value::as_str).unwrap_or("lines");
    burin_map_core::script::water_from(&v, Some(kind)).map_err(|e| fail(ctx, e))?;
    to_js(&v, ctx)
}

/// `B.shore(json)`: the shoreline, checked.
fn native_shore(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let v = json_arg(args, 0, ctx)?;
    burin_map_core::script::shore_from(&v).map_err(|e| fail(ctx, e))?;
    to_js(&v, ctx)
}

/// `B.parameters()`: the style catalogue, as JSON.
fn native_parameters(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::from(js_string!(burin_map_core::script::parameters().to_string())))
}

/// `B.atlas(json)`: a whole list, checked, and saved as the document.
fn native_atlas(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let v = json_arg(args, 0, ctx)?;
    let atlas = burin_map_core::script::atlas_from(&v).map_err(|e| fail(ctx, e))?;
    let saved = burin_map_core::script::atlas_to_json(&atlas);
    SAVED.with(|s| *s.borrow_mut() = Some(saved));
    to_js(&v, ctx)
}

/// `B.draw(json)`: the document, checked and saved, ready to draw.
fn native_draw(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let v = json_arg(args, 0, ctx)?;
    let atlas = burin_map_core::script::atlas_from(&v).map_err(|e| fail(ctx, e))?;
    if atlas.batches.is_empty() && atlas.water.is_none() {
        return Err(fail(ctx, "the document has nothing to draw"));
    }
    let saved = burin_map_core::script::atlas_to_json(&atlas);
    SAVED.with(|s| *s.borrow_mut() = Some(saved.clone()));
    to_js(&saved, ctx)
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

fn script_error(err: JsError, source: &str, ctx: &mut Context) -> ScriptError {
    let native = err.try_native(ctx).ok();
    let message = native.as_ref().map(|n| n.message().to_string());
    if let (Some(message), Some((failed, at))) = (&message, FAILURE.with(|f| f.borrow_mut().take())) {
        if *message == failed {
            return ScriptError { message: failed, at };
        }
    }
    let text = err.to_string();
    let first = text.lines().next().unwrap_or_default();
    let kind = native.as_ref().map(|n| format!("{}", n.kind())).unwrap_or_default();
    let message = match (&message, kind.is_empty()) {
        (Some(m), false) => format!("{kind}: {m}"),
        _ => first.split(" (").next().unwrap_or(first).to_string(),
    };
    // `x is not defined` comes without a reliable position: the first use of
    // the name.
    if let Some(name) = message.strip_prefix("ReferenceError: ").and_then(|m| m.strip_suffix(" is not defined")) {
        if let Some(at) = first_use(source, name) {
            return ScriptError { at: Some(at), message };
        }
    }
    // Runtime errors: `TypeError: … (document.js:2:7)`.
    let marker = format!("({SCRIPT_PATH}:");
    if let Some(i) = first.rfind(&marker) {
        let spot = &first[i + marker.len()..first.len().saturating_sub(1)];
        if let Some((line, column)) = spot.split_once(':') {
            if let (Ok(line), Ok(column)) = (line.parse(), column.parse()) {
                return ScriptError { message, at: Some(At(line, column)) };
            }
        }
    }
    // Syntax errors: `… at line 1, col 9`.
    if let Some(i) = first.rfind(" at line ") {
        let spot = &first[i + 9..];
        if let Some((line, column)) = spot.split_once(", col ") {
            let column: String = column.chars().take_while(char::is_ascii_digit).collect();
            if let (Ok(line), Ok(column)) = (line.trim().parse(), column.parse()) {
                let message = format!("SyntaxError: {}", first[..i].trim_start_matches("SyntaxError: "));
                return ScriptError { message, at: Some(At(line, column)) };
            }
        }
    }
    ScriptError { message, at: None }
}

/// The first use of `name` in the user's script.
fn first_use(source: &str, name: &str) -> Option<At> {
    let at = source.find(name)?;
    let (line, column) = line_col(source, at);
    Some(At(line, column))
}

/// The 1-based line and column of a byte offset.
fn line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 1;
    let mut column = 1;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}
