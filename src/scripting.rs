//! Rhai scripting: the engine behind entry conditions and scripts.
//!
//! Conditions and scripts authored on [`DialogueEntry`](crate::data::DialogueEntry)
//! are Rhai code. Both see the variable store as `vars`:
//!
//! ```rhai
//! // a condition:
//! vars["Gold"] >= 10 && !vars["AcceptedJob"]
//!
//! // a script:
//! vars["AcceptedJob"] = true;
//! vars["Gold"] -= 10;
//! ```
//!
//! Reading an unknown variable is an error; `vars.has("name")` tests existence.
//! Writing creates the variable if needed. Numbers are floats on the script
//! side, but Rhai mixes integers and floats freely, so `vars["Gold"] >= 10`
//! works.

use std::collections::HashMap;

use bevy::prelude::*;
use rhai::{AST, Dynamic, Engine, EvalAltResult, ParseError};

use crate::data::{ConversationId, DialogueDatabase, DialogueEntry, EntryId, FieldValue};
use crate::runtime::Variables;

/// The engine that evaluates dialogue conditions and scripts.
#[derive(Resource)]
pub struct ScriptEngine(pub Engine);

impl Default for ScriptEngine {
    fn default() -> Self {
        Self(engine())
    }
}

/// The compiled logic of one entry.
struct CompiledLogic {
    /// The entry's condition, if it has one.
    condition: Option<AST>,
    /// The entry's script, if it has one.
    script: Option<AST>,
}

/// Compiled conditions and scripts of every loaded database, by entry.
#[derive(Resource, Default)]
pub struct CompiledScripts(HashMap<(ConversationId, EntryId), CompiledLogic>);

impl CompiledScripts {
    /// The compiled condition of `key`'s entry, if it has one.
    pub fn condition(&self, key: (ConversationId, EntryId)) -> Option<&AST> {
        self.0.get(&key)?.condition.as_ref()
    }

    /// The compiled script of `key`'s entry, if it has one.
    pub fn script(&self, key: (ConversationId, EntryId)) -> Option<&AST> {
        self.0.get(&key)?.script.as_ref()
    }
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

    compiled.0 = databases
        .iter()
        .flat_map(|(_, db)| &db.conversations)
        .flat_map(|conversation| {
            conversation
                .entries
                .iter()
                .map(|entry| ((conversation.id, entry.id), entry))
        })
        .filter_map(|(key, entry)| Some((key, compile_entry(&engine.0, key, entry)?)))
        .collect();
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
) -> Option<AST> {
    (!text.is_empty())
        .then(|| compile(text))?
        .inspect_err(|error| {
            warn!(
                "{what} on entry {} of conversation {} doesn't compile: {error}",
                key.1.0, key.0.0
            );
        })
        .ok()
}

/// Builds the engine that evaluates dialogue conditions and scripts.
pub fn engine() -> Engine {
    let mut engine = Engine::new();
    engine
        .register_type_with_name::<Variables>("Variables")
        .register_indexer_get(get_variable)
        .register_indexer_set(set_variable)
        .register_fn("has", |vars: &mut Variables, name: &str| {
            vars.get(name).is_some()
        });
    engine
}

/// `vars[name]`: the variable's current value. Unknown names are an error.
fn get_variable(vars: &mut Variables, name: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    match vars.get(name) {
        Some(value) => Ok(to_dynamic(value)),
        None => Err(format!("unknown variable `{name}`").into()),
    }
}

/// `vars[name] = value`: sets the variable, creating it if needed.
fn set_variable(
    vars: &mut Variables,
    name: &str,
    value: Dynamic,
) -> Result<(), Box<EvalAltResult>> {
    match from_dynamic(&value) {
        Some(value) => {
            vars.set(name, value);
            Ok(())
        }
        None => Err(format!(
            "variable `{name}` can't hold a value of type {}",
            value.type_name()
        )
        .into()),
    }
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
    use rhai::Scope;
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
                ],
                ..Default::default()
            }],
            ..Default::default()
        }
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
    fn compiled_conditions_evaluate_against_the_store(mut vars: Variables) {
        let engine = engine();
        let ast = engine.compile_expression(r#"vars["Gold"] >= 10"#).unwrap();
        let mut scope = Scope::new();
        scope.push("vars", std::mem::take(&mut vars));
        assert!(
            engine
                .eval_ast_with_scope::<bool>(&mut scope, &ast)
                .unwrap()
        );
    }

    #[fixture]
    fn vars() -> Variables {
        let mut vars = Variables::default();
        vars.set("Gold", 12.0);
        vars.set("Name", "Feri");
        vars.set("AcceptedJob", false);
        vars
    }

    /// Evaluates `code` with the store exposed as `vars`, moving it in and out.
    fn eval<T: Clone + Send + Sync + 'static>(
        code: &str,
        vars: &mut Variables,
    ) -> Result<T, Box<EvalAltResult>> {
        let engine = engine();
        let mut scope = Scope::new();
        scope.push("vars", std::mem::take(vars));
        let result = engine.eval_with_scope::<T>(&mut scope, code);
        *vars = scope.remove("vars").expect("the store stays in scope");
        result
    }

    #[rstest]
    fn conditions_compare_numbers_with_int_literals(mut vars: Variables) {
        assert!(eval::<bool>(r#"vars["Gold"] >= 10"#, &mut vars).unwrap());
        vars.set("Gold", 5.0);
        assert!(!eval::<bool>(r#"vars["Gold"] >= 10"#, &mut vars).unwrap());
    }

    #[rstest]
    fn conditions_read_text_and_bools(mut vars: Variables) {
        assert!(
            eval::<bool>(
                r#"vars["Name"] == "Feri" && !vars["AcceptedJob"]"#,
                &mut vars
            )
            .unwrap()
        );
    }

    #[rstest]
    fn scripts_write_back_to_the_store(mut vars: Variables) {
        eval::<()>(
            r#"vars["AcceptedJob"] = true; vars["Gold"] += 30; vars["Greeting"] = "hi";"#,
            &mut vars,
        )
        .unwrap();
        assert!(vars.truthy("AcceptedJob"));
        assert_eq!(vars.number("Gold"), 42.0);
        assert_eq!(vars.text("Greeting"), "hi");
    }

    #[rstest]
    fn reading_an_unknown_variable_is_an_error(mut vars: Variables) {
        let error = eval::<bool>(r#"vars["Nope"]"#, &mut vars).unwrap_err();
        assert!(error.to_string().contains("unknown variable `Nope`"));
    }

    #[rstest]
    fn has_tests_existence(mut vars: Variables) {
        assert!(eval::<bool>(r#"vars.has("Gold") && !vars.has("Nope")"#, &mut vars).unwrap());
    }
}
