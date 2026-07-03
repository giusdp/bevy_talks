//! Rhai scripting: the engine behind entry conditions and scripts.
//!
//! Conditions and scripts authored on [`DialogueEntry`]
//! are Rhai code. Both see the variable store as `vars` and can call any
//! system the game registered with
//! [`add_dialogue_system`](AddDialogueSystem::add_dialogue_system):
//!
//! ```rhai
//! // a condition:
//! vars["Gold"] >= 10 && has_item("sword")
//!
//! // a script:
//! vars["AcceptedJob"] = true;
//! give_item("sword");
//! ```
//!
//! Reading an unknown variable is an error; `vars.has("name")` tests existence.
//! Writing creates the variable if needed. Numbers are floats on the script
//! side, but Rhai mixes integers and floats freely, so `vars["Gold"] >= 10`
//! works.
//!
//! Broken logic never blocks dialogue: a condition that fails to compile or
//! errors at runtime passes (with a warning), a failing script is reported
//! and skipped.

pub mod functions;

pub use functions::{AddDialogueSystem, DialogueSystems, ScriptArg, ScriptArgs, ScriptReturn};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bevy::prelude::*;
use rhai::{AST, Dynamic, Engine, EvalAltResult, ParseError, Scope};

use crate::data::{ConversationId, DialogueDatabase, DialogueEntry, EntryId, FieldValue};
use crate::runtime::Variables;
use functions::{WORLD, with_world};

/// The engine that evaluates dialogue conditions and scripts.
///
/// Rebuilt whenever the game's registered dialogue systems change.
#[derive(Resource)]
pub struct ScriptEngine(pub Arc<Engine>);

impl Default for ScriptEngine {
    fn default() -> Self {
        Self(Arc::new(build_engine(&DialogueSystems::default())))
    }
}

/// Rebuilds the engine from the registered dialogue systems.
///
/// Runs when [`DialogueSystems`] changes; compiled ASTs stay valid because
/// Rhai resolves function calls at evaluation time.
pub fn rebuild_engine(systems: Res<DialogueSystems>, mut engine: ResMut<ScriptEngine>) {
    engine.0 = Arc::new(build_engine(&systems));
}

/// Builds an engine with the store bindings and the game's dialogue systems.
fn build_engine(systems: &DialogueSystems) -> Engine {
    let mut engine = Engine::new();
    engine
        .register_type_with_name::<VarStore>("Variables")
        .register_indexer_get(get_variable)
        .register_indexer_set(set_variable)
        .register_fn("has", has_variable);
    systems.install_into(&mut engine);
    engine
}

/// The compiled logic of one entry.
struct CompiledLogic {
    /// The entry's condition, if it has one.
    condition: Option<Arc<AST>>,
    /// The entry's script, if it has one.
    script: Option<Arc<AST>>,
}

/// Compiled conditions and scripts of every loaded database, by entry.
#[derive(Resource, Default)]
pub struct CompiledScripts {
    /// Compiled logic by entry.
    logic: HashMap<(ConversationId, EntryId), CompiledLogic>,
    /// The databases the logic came from.
    sources: HashSet<AssetId<DialogueDatabase>>,
}

impl CompiledScripts {
    /// The compiled condition of `key`'s entry, if it has one.
    pub fn condition(&self, key: (ConversationId, EntryId)) -> Option<Arc<AST>> {
        self.logic.get(&key)?.condition.clone()
    }

    /// The compiled script of `key`'s entry, if it has one.
    pub fn script(&self, key: (ConversationId, EntryId)) -> Option<Arc<AST>> {
        self.logic.get(&key)?.script.clone()
    }
}

/// Compiles `db` now unless it already has been.
///
/// The runner calls this when a conversation starts: asset events reach
/// [`compile_scripts`] one frame after the asset exists, and a condition on
/// the first line must not race that frame.
pub(crate) fn ensure_compiled(
    world: &mut World,
    id: AssetId<DialogueDatabase>,
    db: &DialogueDatabase,
) {
    let engine = world.resource::<ScriptEngine>().0.clone();
    let mut compiled = world.resource_mut::<CompiledScripts>();
    if !compiled.sources.insert(id) {
        return;
    }
    let logic: Vec<_> = compile_database(&engine, db).collect();
    compiled.logic.extend(logic);
}

/// Compiles conditions and scripts from every database as it loads.
///
/// Conditions compile in expression mode: statements like `vars["x"] = 1`
/// are load-time errors there. Anything that fails to compile is reported
/// and skipped, so a broken condition never blocks dialogue.
pub fn compile_scripts(
    mut events: MessageReader<AssetEvent<DialogueDatabase>>,
    databases: Res<Assets<DialogueDatabase>>,
    engine: Res<ScriptEngine>,
    mut compiled: ResMut<CompiledScripts>,
) {
    let relevant = events.read().any(|event| {
        matches!(
            event,
            AssetEvent::Added { .. } | AssetEvent::Modified { .. }
        )
    });
    if !relevant {
        return;
    }

    compiled.sources = databases.iter().map(|(id, _)| id).collect();
    compiled.logic = databases
        .iter()
        .flat_map(|(_, db)| compile_database(&engine.0, db))
        .collect();
}

/// Compiles every entry of one database that has logic.
fn compile_database<'a>(
    engine: &'a Engine,
    db: &'a DialogueDatabase,
) -> impl Iterator<Item = ((ConversationId, EntryId), CompiledLogic)> + 'a {
    db.conversations
        .iter()
        .flat_map(|conversation| {
            conversation
                .entries
                .iter()
                .map(|entry| ((conversation.id, entry.id), entry))
        })
        .filter_map(move |(key, entry)| Some((key, compile_entry(engine, key, entry)?)))
}

/// Compiles one entry's logic; `None` when the entry has none.
fn compile_entry(
    engine: &Engine,
    key: (ConversationId, EntryId),
    entry: &DialogueEntry,
) -> Option<CompiledLogic> {
    let condition = compile_snippet(&entry.condition, key, "condition", |text| {
        engine.compile_expression(text)
    });
    let script = compile_snippet(&entry.script, key, "script", |text| engine.compile(text));
    (condition.is_some() || script.is_some()).then_some(CompiledLogic { condition, script })
}

/// Compiles one authored snippet, reporting failures. Empty text is no logic.
fn compile_snippet(
    text: &str,
    key: (ConversationId, EntryId),
    what: &str,
    compile: impl FnOnce(&str) -> Result<AST, ParseError>,
) -> Option<Arc<AST>> {
    (!text.is_empty())
        .then(|| compile(text))?
        .inspect_err(|error| {
            warn!(
                "{what} on entry {} of conversation {} doesn't compile: {error}",
                key.1.0, key.0.0
            );
        })
        .ok()
        .map(Arc::new)
}

/// Evaluates `key`'s condition. Entries without one, or with broken logic, pass.
pub fn check_condition(world: &mut World, key: (ConversationId, EntryId)) -> bool {
    let Some(ast) = world.resource::<CompiledScripts>().condition(key) else {
        return true;
    };
    eval_ast(world, &ast)
        .and_then(|value| {
            value
                .as_bool()
                .map_err(|got| format!("expected a bool, got {got}").into())
        })
        .unwrap_or_else(|error| {
            warn!(
                "condition on entry {} of conversation {} failed: {error}",
                key.1.0, key.0.0
            );
            true
        })
}

/// Runs `key`'s script, if it has one. Failures are reported and skipped.
pub fn run_script(world: &mut World, key: (ConversationId, EntryId)) {
    let Some(ast) = world.resource::<CompiledScripts>().script(key) else {
        return;
    };
    if let Err(error) = eval_ast(world, &ast) {
        warn!(
            "script on entry {} of conversation {} failed: {error}",
            key.1.0, key.0.0
        );
    }
}

/// Evaluates a compiled AST with `vars` bound and the world reachable.
fn eval_ast(world: &mut World, ast: &AST) -> Result<Dynamic, Box<EvalAltResult>> {
    let engine = world.resource::<ScriptEngine>().0.clone();
    let mut scope = Scope::new();
    scope.push("vars", VarStore);
    WORLD.set(world, || {
        engine.eval_ast_with_scope::<Dynamic>(&mut scope, ast)
    })
}

/// The `vars` binding scripts see: a handle to the [`Variables`] resource of
/// the world being evaluated.
#[derive(Clone, Copy)]
struct VarStore;

/// `vars[name]`: the variable's current value. Unknown names are an error.
fn get_variable(_: &mut VarStore, name: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    with_world(|world| {
        world
            .resource::<Variables>()
            .get(name)
            .map(to_dynamic)
            .ok_or_else(|| format!("unknown variable `{name}`"))
    })?
    .map_err(Into::into)
}

/// `vars[name] = value`: sets the variable, creating it if needed.
fn set_variable(_: &mut VarStore, name: &str, value: Dynamic) -> Result<(), Box<EvalAltResult>> {
    let Some(converted) = from_dynamic(&value) else {
        return Err(format!(
            "variable `{name}` can't hold a value of type {}",
            value.type_name()
        )
        .into());
    };
    with_world(|world| world.resource_mut::<Variables>().set(name, converted))
}

/// `vars.has(name)`: whether the variable exists.
fn has_variable(_: &mut VarStore, name: &str) -> Result<bool, Box<EvalAltResult>> {
    with_world(|world| world.resource::<Variables>().get(name).is_some())
}

/// A variable value as a script value. Numbers become floats, actors their id.
fn to_dynamic(value: &FieldValue) -> Dynamic {
    match value {
        FieldValue::Text(s) | FieldValue::Localization(s) => s.as_str().into(),
        FieldValue::Number(n) => Dynamic::from_float(f64::from(*n)),
        FieldValue::Boolean(b) => Dynamic::from_bool(*b),
        FieldValue::Actor(id) => Dynamic::from_int(i64::from(id.0)),
    }
}

/// A script value as a variable value: bools, numbers (int or float), text.
fn from_dynamic(value: &Dynamic) -> Option<FieldValue> {
    if let Ok(b) = value.as_bool() {
        return Some(FieldValue::Boolean(b));
    }
    if let Ok(n) = value.as_float() {
        return Some(FieldValue::Number(n as f32));
    }
    if let Ok(n) = value.as_int() {
        return Some(FieldValue::Number(n as f32));
    }
    if value.is_string() {
        return Some(FieldValue::Text(value.clone().into_string().ok()?));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Conversation;
    use rstest::{fixture, rstest};

    #[fixture]
    fn db() -> DialogueDatabase {
        let entry = |id: i32, condition: &str, script: &str| DialogueEntry {
            id: EntryId(id),
            condition: condition.to_owned(),
            script: script.to_owned(),
            ..Default::default()
        };
        DialogueDatabase {
            conversations: vec![Conversation {
                id: ConversationId(1),
                entries: vec![
                    entry(1, "", r#"vars["Greeted"] = true"#),
                    entry(2, r#"vars["Gold"] >= 10"#, ""),
                    entry(3, "vars[", ""),
                    entry(4, r#"vars["x"] = 1"#, ""),
                    entry(5, "", ""),
                    entry(6, r#"vars["Missing"] > 1"#, ""),
                ],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// A world with the resources evaluation needs.
    #[fixture]
    fn world() -> World {
        let mut vars = Variables::default();
        vars.set("Gold", 12.0);
        vars.set("Name", "Feri");
        vars.set("AcceptedJob", false);

        let mut world = World::new();
        world.insert_resource(vars);
        world.init_resource::<ScriptEngine>();
        world.init_resource::<CompiledScripts>();
        world
    }

    /// Compiles and evaluates `code` as dialogue logic against `world`.
    fn eval(code: &str, world: &mut World) -> Result<Dynamic, Box<EvalAltResult>> {
        let engine = world.resource::<ScriptEngine>().0.clone();
        let ast = engine.compile(code).map_err(|error| error.to_string())?;
        eval_ast(world, &ast)
    }

    /// Evaluates `code` and expects a boolean result.
    fn eval_bool(code: &str, world: &mut World) -> bool {
        eval(code, world).unwrap().as_bool().unwrap()
    }

    #[rstest]
    fn conditions_compare_numbers_with_int_literals(mut world: World) {
        assert!(eval_bool(r#"vars["Gold"] >= 10"#, &mut world));
        world.resource_mut::<Variables>().set("Gold", 5.0);
        assert!(!eval_bool(r#"vars["Gold"] >= 10"#, &mut world));
    }

    #[rstest]
    fn conditions_read_text_and_bools(mut world: World) {
        assert!(eval_bool(
            r#"vars["Name"] == "Feri" && !vars["AcceptedJob"]"#,
            &mut world
        ));
    }

    #[rstest]
    fn scripts_write_back_to_the_store(mut world: World) {
        let _ = eval(
            r#"vars["AcceptedJob"] = true; vars["Gold"] += 30; vars["Greeting"] = "hi";"#,
            &mut world,
        )
        .unwrap();
        let vars = world.resource::<Variables>();
        assert!(vars.truthy("AcceptedJob"));
        assert_eq!(vars.number("Gold"), 42.0);
        assert_eq!(vars.text("Greeting"), "hi");
    }

    #[rstest]
    fn reading_an_unknown_variable_is_an_error(mut world: World) {
        let error = eval(r#"vars["Nope"]"#, &mut world).unwrap_err();
        assert!(error.to_string().contains("unknown variable `Nope`"));
    }

    #[rstest]
    fn has_tests_existence(mut world: World) {
        assert!(eval_bool(
            r#"vars.has("Gold") && !vars.has("Nope")"#,
            &mut world
        ));
    }

    #[rstest]
    fn loading_a_database_compiles_its_logic(db: DialogueDatabase) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), crate::TalksPlugin));
        let _handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueDatabase>>()
            .add(db);
        // Asset events land after Update; the compiler sees them next frame.
        app.update();
        app.update();

        let compiled = app.world().resource::<CompiledScripts>();
        let key = |id| (ConversationId(1), EntryId(id));
        assert!(compiled.script(key(1)).is_some());
        assert!(compiled.condition(key(1)).is_none());
        assert!(compiled.condition(key(2)).is_some());
        assert!(
            compiled.condition(key(3)).is_none(),
            "a broken condition is reported and skipped"
        );
        assert!(
            compiled.condition(key(4)).is_none(),
            "statements don't compile as conditions"
        );
        assert!(compiled.script(key(5)).is_none());
    }

    #[rstest]
    fn conditions_gate_entries_and_broken_logic_passes(db: DialogueDatabase) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), crate::TalksPlugin));
        let _handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueDatabase>>()
            .add(db);
        app.update();
        app.update();

        let world = app.world_mut();
        world.resource_mut::<Variables>().set("Gold", 12.0);
        let key = |id| (ConversationId(1), EntryId(id));
        assert!(check_condition(world, key(2)));
        world.resource_mut::<Variables>().set("Gold", 5.0);
        assert!(!check_condition(world, key(2)));
        assert!(check_condition(world, key(5)), "no condition passes");
        assert!(check_condition(world, key(3)), "broken condition passes");
        assert!(
            check_condition(world, key(6)),
            "runtime errors pass with a warning"
        );

        run_script(world, key(1));
        assert!(world.resource::<Variables>().truthy("Greeted"));
    }

    /// Counts what `give_item` handed out in the dialogue-systems test.
    #[derive(Resource, Default)]
    struct Given(u32);

    #[rstest]
    fn dialogue_systems_run_with_world_access(mut world: World) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), crate::TalksPlugin));
        app.init_resource::<Given>();
        app.add_dialogue_system("double", |In(n): In<f64>| n * 2.0);
        app.add_dialogue_system(
            "give_item",
            |In(name): In<String>, mut given: ResMut<Given>| {
                assert_eq!(name, "sword");
                given.0 += 1;
            },
        );
        // First update rebuilds the engine with the registered systems.
        app.update();

        app.world_mut()
            .resource_mut::<Variables>()
            .set("Gold", 12.0);
        assert!(eval_bool(
            r#"give_item("sword"); double(vars["Gold"]) >= 24"#,
            app.world_mut(),
        ));
        assert_eq!(app.world().resource::<Given>().0, 1);

        // The fixture world has no registered systems; the call must error.
        let error = eval(r#"double(2.0) == 4.0"#, &mut world).unwrap_err();
        assert!(error.to_string().contains("double"));
    }
}
