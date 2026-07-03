//! Cues: the staging layer behind entry sequences.
//!
//! A sequence is Rhai code where command calls schedule **cues** instead of
//! executing on the spot. Evaluating the sequence produces the cue list; a
//! driver plays it out over time:
//!
//! ```rhai
//! emit("scene_start");
//! wait(2.0).emits("looked");
//! wait(1.0).after("looked").required();
//! wait(line_end)
//! ```
//!
//! Every cue supports the timing methods `at(seconds)` (start after a delay),
//! `after(message)` (start when a message fires), `emits(message)` (fire a
//! message when done), and `required()` (still runs when the line is
//! skipped). `wait` and `emit` are built in; everything else comes from
//! commands the game registers. `line_end` is the estimated reading time of
//! the line being presented.

use bevy::prelude::*;
use rhai::{AST, Dynamic, Engine, EvalAltResult, Scope};

use super::functions::with_world;

/// One scheduled cue: a command call plus its timing.
#[derive(Debug, Clone, Default)]
pub struct CueRecord {
    /// The command to run.
    pub name: String,
    /// The arguments it was scheduled with.
    pub args: Vec<Dynamic>,
    /// Seconds into the sequence before the cue starts.
    pub at: f32,
    /// Message that must fire before the cue starts.
    pub after: Option<String>,
    /// Message fired when the cue finishes.
    pub emits: Option<String>,
    /// Whether the cue still runs when the sequence is skipped.
    pub required: bool,
}

/// The cue list being built while a sequence script evaluates.
#[derive(Resource, Default)]
pub struct PendingCues(pub(crate) Vec<CueRecord>);

/// The script-side handle to a scheduled cue; the timing methods live here.
#[derive(Clone, Copy)]
pub(crate) struct CueRef(usize);

/// Registers the cue type, its timing methods, and the built-in commands.
pub(crate) fn install(engine: &mut Engine) {
    engine
        .register_type_with_name::<CueRef>("Cue")
        .register_fn("at", |cue: &mut CueRef, secs: f64| {
            update(*cue, |record| record.at = secs as f32)
        })
        .register_fn("at", |cue: &mut CueRef, secs: i64| {
            update(*cue, |record| record.at = secs as f32)
        })
        .register_fn("after", |cue: &mut CueRef, message: &str| {
            update(*cue, |record| record.after = Some(message.to_owned()))
        })
        .register_fn("emits", |cue: &mut CueRef, message: &str| {
            update(*cue, |record| record.emits = Some(message.to_owned()))
        })
        .register_fn("required", |cue: &mut CueRef| {
            update(*cue, |record| record.required = true)
        })
        .register_fn("wait", |secs: f64| {
            schedule("wait".to_owned(), vec![Dynamic::from_float(secs)])
        })
        .register_fn("wait", |secs: i64| {
            schedule("wait".to_owned(), vec![Dynamic::from_float(secs as f64)])
        })
        .register_fn("emit", |message: &str| {
            schedule("emit".to_owned(), vec![message.into()])
        });
}

/// Schedules a cue, returning the handle the timing methods chain on.
pub(crate) fn schedule(name: String, args: Vec<Dynamic>) -> Result<CueRef, Box<EvalAltResult>> {
    with_world(|world| {
        let mut pending = world.resource_mut::<PendingCues>();
        pending.0.push(CueRecord {
            name,
            args,
            ..Default::default()
        });
        CueRef(pending.0.len() - 1)
    })
}

/// Applies `change` to the cue's record.
fn update(cue: CueRef, change: impl FnOnce(&mut CueRecord)) -> Result<CueRef, Box<EvalAltResult>> {
    with_world(|world| {
        world
            .resource_mut::<PendingCues>()
            .0
            .get_mut(cue.0)
            .map(change)
    })?
    .ok_or("cue no longer pending")?;
    Ok(cue)
}

/// Evaluates a compiled sequence into its cue list.
///
/// `line_end` is the reading time of the line being presented, available to
/// the script under that name.
pub fn eval_cues(
    world: &mut World,
    ast: &AST,
    line_end: f32,
) -> Result<Vec<CueRecord>, Box<EvalAltResult>> {
    world.resource_mut::<PendingCues>().0.clear();
    let mut scope = Scope::new();
    scope.push_constant("line_end", f64::from(line_end));
    let _ = super::eval_ast_in(world, ast, scope)?;
    Ok(std::mem::take(&mut world.resource_mut::<PendingCues>().0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Variables;
    use crate::scripting::{CompiledScripts, ScriptEngine};
    use rstest::{fixture, rstest};

    /// A world with the resources sequence evaluation needs.
    #[fixture]
    fn world() -> World {
        let mut vars = Variables::default();
        vars.set("Gold", 12.0);

        let mut world = World::new();
        world.insert_resource(vars);
        world.init_resource::<ScriptEngine>();
        world.init_resource::<CompiledScripts>();
        world.init_resource::<PendingCues>();
        world
    }

    /// Compiles and evaluates `code` as a sequence against `world`.
    fn cues(code: &str, world: &mut World) -> Result<Vec<CueRecord>, Box<EvalAltResult>> {
        let engine = world.resource::<ScriptEngine>().0.clone();
        let ast = engine.compile(code).map_err(|error| error.to_string())?;
        eval_cues(world, &ast, 3.0)
    }

    #[rstest]
    fn sequences_schedule_cues_in_order(mut world: World) {
        let cues = cues(r#"emit("go"); wait(2)"#, &mut world).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].name, "emit");
        assert_eq!(cues[0].args[0].clone().into_string().unwrap(), "go");
        assert_eq!(cues[1].name, "wait");
        assert_eq!(cues[1].args[0].as_float().unwrap(), 2.0);
    }

    #[rstest]
    fn timing_methods_chain_and_mutate_the_record(mut world: World) {
        let cues = cues(
            r#"wait(1.0).at(2.0).emits("done").required(); emit("x").after("done")"#,
            &mut world,
        )
        .unwrap();
        assert_eq!(cues[0].at, 2.0);
        assert_eq!(cues[0].emits.as_deref(), Some("done"));
        assert!(cues[0].required);
        assert_eq!(cues[1].after.as_deref(), Some("done"));
        assert!(!cues[1].required);
    }

    #[rstest]
    fn sequences_use_vars_and_line_end(mut world: World) {
        let cues = cues(
            r#"if vars["Gold"] >= 10 { emit("rich") }; wait(line_end)"#,
            &mut world,
        )
        .unwrap();
        assert_eq!(cues[0].name, "emit");
        assert_eq!(cues[1].args[0].as_float().unwrap(), 3.0);
    }

    #[rstest]
    fn unknown_commands_error(mut world: World) {
        let error = cues(r#"camera("Wide")"#, &mut world).unwrap_err();
        assert!(error.to_string().contains("camera"));
    }
}
