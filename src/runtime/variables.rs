//! The variable store: mutable game state that dialogue reads and writes.
//!
//! [`Variables`] is a plain resource holding the current value of every variable by name.
//!  Loading a [`DialogueDatabase`] seeds it with the database's initial values, without overwriting values that already exist.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::data::{DialogueDatabase, FieldValue};

/// Current variable values, keyed by name.
///
/// Also available to conditions and scripts as `var`, see [`crate::scripting`].
/// Clone exists because the store moves in and out of script scopes; it is
/// not meant for keeping copies around.
#[derive(Resource, Debug, Default, Clone)]
pub struct Variables(pub HashMap<String, FieldValue>);

impl Variables {
    /// The current value of `name`, if it exists.
    pub fn get(&self, name: &str) -> Option<&FieldValue> {
        self.0.get(name)
    }

    /// Sets `name` to `value`, creating the variable if needed.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<FieldValue>) {
        self.0.insert(name.into(), value.into());
    }

    /// The boolean value of `name`; `false` if missing or not a boolean.
    pub fn truthy(&self, name: &str) -> bool {
        matches!(self.get(name), Some(FieldValue::Boolean(true)))
    }

    /// The numeric value of `name`; `0.0` if missing or not a number.
    pub fn number(&self, name: &str) -> f32 {
        match self.get(name) {
            Some(FieldValue::Number(n)) => *n,
            _ => 0.0,
        }
    }

    /// The text value of `name`; `""` if missing or not text.
    pub fn text(&self, name: &str) -> &str {
        match self.get(name) {
            Some(FieldValue::Text(s)) => s,
            _ => "",
        }
    }

    /// Adds `db`'s variables that aren't in the store yet, at their initial values.
    pub fn seed(&mut self, db: &DialogueDatabase) {
        for variable in &db.variables {
            self.0
                .entry(variable.name.clone())
                .or_insert_with(|| variable.initial.clone());
        }
    }
}

/// Seeds [`Variables`] from every database as it loads.
pub fn seed_variables(
    mut events: MessageReader<AssetEvent<DialogueDatabase>>,
    databases: Res<Assets<DialogueDatabase>>,
    mut variables: ResMut<Variables>,
) {
    for event in events.read() {
        let (AssetEvent::Added { id } | AssetEvent::Modified { id }) = event else {
            continue;
        };
        if let Some(db) = databases.get(*id) {
            variables.seed(db);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Variable;
    use rstest::{fixture, rstest};

    #[fixture]
    fn db() -> DialogueDatabase {
        DialogueDatabase {
            variables: vec![
                Variable {
                    name: "AcceptedJob".to_owned(),
                    initial: FieldValue::Boolean(false),
                    fields: vec![],
                },
                Variable {
                    name: "Gold".to_owned(),
                    initial: FieldValue::Number(10.0),
                    fields: vec![],
                },
            ],
            ..Default::default()
        }
    }

    #[rstest]
    fn seed_fills_missing_variables(db: DialogueDatabase) {
        let mut vars = Variables::default();
        vars.seed(&db);
        assert!(!vars.truthy("AcceptedJob"));
        assert_eq!(vars.number("Gold"), 10.0);
    }

    #[rstest]
    fn seed_keeps_existing_values(db: DialogueDatabase) {
        let mut vars = Variables::default();
        vars.set("Gold", 99.0);
        vars.seed(&db);
        assert_eq!(vars.number("Gold"), 99.0);
        assert!(!vars.truthy("AcceptedJob"));
    }

    #[test]
    fn typed_accessors_default_on_missing_or_mismatch() {
        let mut vars = Variables::default();
        vars.set("Name", "Feri");
        assert_eq!(vars.text("Name"), "Feri");
        assert_eq!(vars.text("Missing"), "");
        assert_eq!(vars.number("Name"), 0.0);
        assert!(!vars.truthy("Name"));
    }

    #[rstest]
    fn loading_a_database_seeds_the_store(db: DialogueDatabase) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), crate::TalksPlugin));
        let _handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueDatabase>>()
            .add(db);
        // Asset events land after Update; the seeder sees them next frame.
        app.update();
        app.update();

        let vars = app.world().resource::<Variables>();
        assert_eq!(vars.number("Gold"), 10.0);
    }
}
