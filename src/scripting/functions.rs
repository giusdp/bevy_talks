//! Game functions callable from conditions and scripts.
//!
//! The game registers Bevy systems under script-facing names:
//!
//! ```rust,ignore
//! app.add_dialogue_system("has_item", |In(name): In<String>, inventory: Res<Inventory>| {
//!     inventory.contains(&name)
//! });
//! ```
//!
//! A condition can then say `has_item("sword")` and a script `give_item("sword")`.
//! The function runs as a one-shot system in the middle of evaluation, with
//! full world access: queries, resources, commands.

use std::sync::Arc;

use bevy::prelude::*;
use rhai::{Dynamic, Engine, EvalAltResult};
use scoped_tls_hkt::scoped_thread_local;

scoped_thread_local!(
    /// The world evaluating dialogue logic right now.
    pub(crate) static mut WORLD: for<'a> &'a mut World
);

/// Runs `f` against the world evaluating dialogue logic.
///
/// Errors when no evaluation is in progress, e.g. when script code runs
/// outside the dialogue runtime.
pub(crate) fn with_world<R>(f: impl FnOnce(&mut World) -> R) -> Result<R, Box<EvalAltResult>> {
    if !WORLD.is_set() {
        return Err("dialogue logic evaluated outside the dialogue runtime".into());
    }
    Ok(WORLD.with(f))
}

/// Converts script arguments, runs the game system, converts the result.
type Bridge = Arc<dyn Fn(&mut World, Vec<Dynamic>) -> Result<Dynamic, String> + Send + Sync>;

/// One registered dialogue system.
struct DialogueSystem {
    /// The name scripts call.
    name: String,
    /// How many arguments it takes.
    arity: usize,
    /// The typed path into the game's system.
    bridge: Bridge,
}

/// Every system the game registered for use in dialogue logic.
#[derive(Resource, Default)]
pub struct DialogueSystems(Vec<DialogueSystem>);

impl DialogueSystems {
    /// Registers all systems' shims with `engine`.
    pub(crate) fn install_into(&self, engine: &mut Engine) {
        for system in &self.0 {
            install(engine, system);
        }
    }
}

/// Registers one system's shim with the engine.
fn install(engine: &mut Engine, system: &DialogueSystem) {
    let bridge = system.bridge.clone();
    let called = system.name.clone();
    let call = move |args: Vec<Dynamic>| -> Result<Dynamic, Box<EvalAltResult>> {
        with_world(|world| bridge(world, args))?
            .map_err(|error| format!("{called}: {error}").into())
    };
    let name = system.name.as_str();
    match system.arity {
        0 => {
            engine.register_fn(name, move || call(Vec::new()));
        }
        1 => {
            engine.register_fn(name, move |a: Dynamic| call(vec![a]));
        }
        2 => {
            engine.register_fn(name, move |a: Dynamic, b: Dynamic| call(vec![a, b]));
        }
        3 => {
            engine.register_fn(name, move |a: Dynamic, b: Dynamic, c: Dynamic| {
                call(vec![a, b, c])
            });
        }
        4 => {
            engine.register_fn(
                name,
                move |a: Dynamic, b: Dynamic, c: Dynamic, d: Dynamic| call(vec![a, b, c, d]),
            );
        }
        n => warn!("dialogue system `{name}` takes {n} arguments; the limit is 4"),
    }
}

/// A single value passed from script to a dialogue system.
pub trait ScriptArg: Sized {
    /// Converts the script-side value; errors name the expected type.
    fn from_dynamic(value: Dynamic) -> Result<Self, String>;
}

impl ScriptArg for bool {
    fn from_dynamic(value: Dynamic) -> Result<Self, String> {
        value
            .as_bool()
            .map_err(|got| format!("expected a bool, got {got}"))
    }
}

impl ScriptArg for i64 {
    fn from_dynamic(value: Dynamic) -> Result<Self, String> {
        value
            .as_int()
            .map_err(|got| format!("expected an integer, got {got}"))
    }
}

impl ScriptArg for f64 {
    fn from_dynamic(value: Dynamic) -> Result<Self, String> {
        value
            .as_float()
            .or_else(|_| value.as_int().map(|n| n as f64))
            .map_err(|got| format!("expected a number, got {got}"))
    }
}

impl ScriptArg for f32 {
    fn from_dynamic(value: Dynamic) -> Result<Self, String> {
        f64::from_dynamic(value).map(|n| n as f32)
    }
}

impl ScriptArg for String {
    fn from_dynamic(value: Dynamic) -> Result<Self, String> {
        value
            .into_string()
            .map_err(|got| format!("expected a string, got {got}"))
    }
}

impl ScriptArg for Dynamic {
    fn from_dynamic(value: Dynamic) -> Result<Self, String> {
        Ok(value)
    }
}

/// The full argument list of a dialogue system.
pub trait ScriptArgs: Sized {
    /// How many arguments the script must pass.
    const ARITY: usize;

    /// Converts the argument list; length is guaranteed to match [`Self::ARITY`].
    fn from_args(args: Vec<Dynamic>) -> Result<Self, String>;
}

impl ScriptArgs for () {
    const ARITY: usize = 0;

    fn from_args(_: Vec<Dynamic>) -> Result<Self, String> {
        Ok(())
    }
}

/// Implements [`ScriptArgs`] for systems taking a single value.
macro_rules! single_script_arg {
    ($($ty:ty),*) => {$(
        impl ScriptArgs for $ty {
            const ARITY: usize = 1;

            fn from_args(mut args: Vec<Dynamic>) -> Result<Self, String> {
                ScriptArg::from_dynamic(args.remove(0))
            }
        }
    )*};
}

single_script_arg!(bool, i64, f64, f32, String, Dynamic);

/// Implements [`ScriptArgs`] for tuples of [`ScriptArg`]s.
macro_rules! tuple_script_args {
    ($count:literal: $($ty:ident),*) => {
        impl<$($ty: ScriptArg),*> ScriptArgs for ($($ty,)*) {
            const ARITY: usize = $count;

            fn from_args(args: Vec<Dynamic>) -> Result<Self, String> {
                let mut args = args.into_iter();
                Ok(($($ty::from_dynamic(args.next().expect("arity checked"))?,)*))
            }
        }
    };
}

tuple_script_args!(2: A, B);
tuple_script_args!(3: A, B, C);
tuple_script_args!(4: A, B, C, D);

/// A dialogue system's result, as the script sees it.
pub trait ScriptReturn {
    /// Converts to the script-side value.
    fn into_dynamic(self) -> Dynamic;
}

impl ScriptReturn for () {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::UNIT
    }
}

impl ScriptReturn for bool {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::from_bool(self)
    }
}

impl ScriptReturn for i64 {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::from_int(self)
    }
}

impl ScriptReturn for f64 {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::from_float(self)
    }
}

impl ScriptReturn for f32 {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::from_float(f64::from(self))
    }
}

impl ScriptReturn for String {
    fn into_dynamic(self) -> Dynamic {
        self.into()
    }
}

impl ScriptReturn for Dynamic {
    fn into_dynamic(self) -> Dynamic {
        self
    }
}

/// App extension for making game systems callable from dialogue logic.
pub trait AddDialogueSystem {
    /// Makes `system` callable from conditions and scripts as `name`.
    ///
    /// The system's `In` input is the argument list (a value or a tuple, up
    /// to four [`ScriptArg`]s); its return value, if any, becomes the call's
    /// result in the script.
    fn add_dialogue_system<S, I, O, M>(&mut self, name: impl Into<String>, system: S) -> &mut Self
    where
        S: IntoSystem<In<I>, O, M> + 'static,
        I: ScriptArgs + Send + Sync + 'static,
        O: ScriptReturn + Send + Sync + 'static;
}

impl AddDialogueSystem for App {
    fn add_dialogue_system<S, I, O, M>(&mut self, name: impl Into<String>, system: S) -> &mut Self
    where
        S: IntoSystem<In<I>, O, M> + 'static,
        I: ScriptArgs + Send + Sync + 'static,
        O: ScriptReturn + Send + Sync + 'static,
    {
        let id = self.world_mut().register_system(system);
        let bridge: Bridge = Arc::new(move |world, args| {
            let input = I::from_args(args)?;
            world
                .run_system_with(id, input)
                .map(ScriptReturn::into_dynamic)
                .map_err(|error| error.to_string())
        });
        self.init_resource::<DialogueSystems>();
        self.world_mut()
            .resource_mut::<DialogueSystems>()
            .0
            .push(DialogueSystem {
                name: name.into(),
                arity: I::ARITY,
                bridge,
            });
        self
    }
}
