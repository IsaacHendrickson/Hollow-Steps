//! The panel's state for the selected entity definition: its text fields and the editing
//! state of its body, behavior, states and key values.
use super::EditorSession;
use crate::SearchList;

#[derive(Default)]
pub(crate) struct PropertyUi {
    pub(crate) path: Option<std::path::PathBuf>,
    pub(crate) fields: EntityFields,
    /// The entity's death time, edited with a drag box rather than a text field.
    pub(crate) death_secs: f32,
    pub(crate) body: Option<hollow_steps::body::BodyVisual>,
    pub(crate) behavior: String,
    pub(crate) behavior_list: SearchList,
    pub(crate) behavior_name: String,
    pub(crate) new_pose_name: String,
    pub(crate) states: SearchList,
    pub(crate) loaded_state: Option<(String, usize)>,
    pub(crate) state_name: String,
    pub(crate) actions: String,
    pub(crate) transitions: String,
    pub(crate) new_state: String,
    pub(crate) error: Option<String>,
    pub(crate) structured: hollow_steps::definition_editor::StateInputs,
    pub(crate) add_action: SearchList,
    pub(crate) add_condition: SearchList,
    pub(crate) conditions: Vec<SearchList>,
    pub(crate) advanced: bool,
    pub(crate) rows: bool,
    pub(crate) presets: SearchList,
    pub(crate) creation: Option<BehaviorCreation>,
    pub(crate) shared_edit: Option<String>,
    pub(crate) key_values: Option<(String, hollow_steps::behavior_values::KeyValues)>,
}

pub(crate) enum BehaviorSource {
    Preset(usize),
    Copy(String),
}

pub(crate) struct BehaviorCreation {
    pub(crate) source: BehaviorSource,
    pub(crate) name: String,
    pub(crate) assign: bool,
}

impl PropertyUi {
    pub(crate) fn for_entity(editor: &EditorSession, path: Option<std::path::PathBuf>) -> Self {
        let mut ui = Self {
            path,
            ..Self::default()
        };
        if let Some(entity) = ui
            .path
            .as_ref()
            .and_then(|path| editor.content.entities.get(path))
        {
            ui.body = entity.body.clone();
            ui.death_secs = entity.death_secs;
            ui.fields = EntityFields::from_definition(entity);
            ui.behavior_list.selected = editor
                .content
                .behaviors
                .keys()
                .position(|name| name == &entity.behavior);
            ui.rows = editor
                .content
                .behaviors
                .get(&entity.behavior)
                .is_some_and(|b| {
                    b.states
                        .iter()
                        .all(|s| s.actions.is_empty() && s.transitions.is_empty())
                });
        }
        ui
    }
}

/// The Entity section's text boxes, one named field each. They stay text while being edited so a
/// half-typed value is not lost; `to_definition` turns them into a validated definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct EntityFields {
    pub(super) name: String,
    pub(super) health: String,
    pub(super) width: String,
    pub(super) height: String,
    pub(super) contact_damage: String,
    /// The assigned behavior's file stem, chosen from the Behavior list rather than typed.
    pub(super) behavior: String,
}

/// A text field of the Entity section, in the order they are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EntityField {
    Name,
    Health,
    Width,
    Height,
    ContactDamage,
}

impl EntityField {
    pub(super) const ALL: [Self; 5] = [
        Self::Name,
        Self::Health,
        Self::Width,
        Self::Height,
        Self::ContactDamage,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Health => "Health",
            Self::Width => "Width",
            Self::Height => "Height",
            Self::ContactDamage => "Contact damage",
        }
    }

    /// Where the field sits in the library's positional field list
    /// (`definition_editor::validate_entity_field`); the only place that mapping is written.
    pub(super) fn index(self) -> usize {
        match self {
            Self::Name => 0,
            Self::Health => 1,
            Self::Width => 2,
            Self::Height => 3,
            Self::ContactDamage => 4,
        }
    }
}

impl EntityFields {
    pub(super) fn from_definition(entity: &hollow_steps::enemy::EnemyDefinition) -> Self {
        Self {
            name: entity.name.clone(),
            health: entity.health.to_string(),
            width: entity.size.0.to_string(),
            height: entity.size.1.to_string(),
            contact_damage: entity.contact_damage.to_string(),
            behavior: entity.behavior.clone(),
        }
    }

    pub(super) fn text(&self, field: EntityField) -> &str {
        match field {
            EntityField::Name => &self.name,
            EntityField::Health => &self.health,
            EntityField::Width => &self.width,
            EntityField::Height => &self.height,
            EntityField::ContactDamage => &self.contact_damage,
        }
    }

    pub(super) fn text_mut(&mut self, field: EntityField) -> &mut String {
        match field {
            EntityField::Name => &mut self.name,
            EntityField::Health => &mut self.health,
            EntityField::Width => &mut self.width,
            EntityField::Height => &mut self.height,
            EntityField::ContactDamage => &mut self.contact_damage,
        }
    }

    /// Whether one field's text is acceptable, by the same rules as saving.
    pub(super) fn validate(&self, field: EntityField) -> Result<(), String> {
        hollow_steps::definition_editor::validate_entity_field(field.index(), self.text(field))
    }

    /// The definition these fields describe, with the body and death time kept as they are.
    pub(super) fn to_definition(
        &self,
        body: Option<hollow_steps::body::BodyVisual>,
        death_secs: f32,
    ) -> Result<hollow_steps::enemy::EnemyDefinition, String> {
        let fields = [
            self.name.clone(),
            self.health.clone(),
            self.width.clone(),
            self.height.clone(),
            self.contact_damage.clone(),
            self.behavior.clone(),
        ];
        hollow_steps::definition_editor::entity_with_body_and_death(&fields, body, death_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::editing_game;
    use super::*;

    #[test]
    fn fields_round_trip_an_entity_definition_with_its_body_and_death_time() {
        let mut game = editing_game();
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("chase").unwrap();
        let entity = editor.content.entities.get_mut(&path).unwrap();
        entity.body = Some(hollow_steps::body::BodyVisual::for_template("small"));
        entity.death_secs = 1.25;
        let entity = entity.clone();
        let fields = EntityFields::from_definition(&entity);
        assert_eq!(fields.name, entity.name);
        assert_eq!(fields.behavior, entity.behavior);
        assert_eq!(
            fields.to_definition(entity.body.clone(), entity.death_secs),
            Ok(entity)
        );
    }

    #[test]
    fn each_field_is_checked_by_its_own_rule() {
        let cases = [
            (EntityField::Name, "", "goblin"),
            (EntityField::Health, "0", "255"),
            (EntityField::Width, "0", "24"),
            (EntityField::Height, "-3", "24.5"),
            (EntityField::ContactDamage, "256", "0"),
        ];
        assert_eq!(cases.map(|(field, _, _)| field), EntityField::ALL);
        for (field, invalid, valid) in cases {
            let mut fields = EntityFields::default();
            *fields.text_mut(field) = invalid.into();
            assert!(fields.validate(field).is_err(), "{field:?}: {invalid:?}");
            *fields.text_mut(field) = valid.into();
            assert!(fields.validate(field).is_ok(), "{field:?}: {valid:?}");
            for other in EntityField::ALL.into_iter().filter(|other| *other != field) {
                assert_eq!(fields.text(other), "", "{field:?} wrote into {other:?}");
            }
        }
    }

    #[test]
    fn a_bad_field_stops_the_definition_and_names_nothing_else() {
        let mut game = editing_game();
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("chase").unwrap();
        let entity = editor.content.entities[&path].clone();
        let mut fields = EntityFields::from_definition(&entity);
        fields.health = "lots".into();
        assert!(fields.to_definition(None, entity.death_secs).is_err());
        assert_eq!(fields.validate(EntityField::Name), Ok(()));
        assert!(fields.validate(EntityField::Health).is_err());
    }
}
