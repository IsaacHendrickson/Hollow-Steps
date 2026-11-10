//! Pure boss text buffers and ordered attack/action edits. Invalid input never changes a definition.
use crate::behavior::Action;
use crate::boss_definition::{AttackDefinition, AttackSelection, BossDefinition};
use crate::definition_editor::{move_row, remove_row};

#[derive(Clone, Debug, PartialEq)]
pub struct BossActionInput {
    pub kind: usize,
    pub values: Vec<String>,
}

impl BossActionInput {
    pub fn catalog() -> Vec<(usize, &'static str)> {
        Action::TYPES
            .iter()
            .enumerate()
            .map(|(kind, name)| (kind, *name))
            .collect()
    }

    pub fn new(kind: usize) -> Result<Self, String> {
        let action = Action::defaults(kind).ok_or("unknown action")?;
        crate::boss_definition::validate_action(&action)?;
        Ok(Self::from_action(&action))
    }

    pub fn from_action(action: &Action) -> Self {
        Self {
            kind: action.kind(),
            values: action.parameters().into_iter().map(|(_, v)| v).collect(),
        }
    }

    pub fn build(&self) -> Result<Action, String> {
        let mut action = Action::defaults(self.kind).ok_or("unknown action")?;
        if self.values.len() != action.parameters().len() {
            return Err("missing action parameter".into());
        }
        for (i, value) in self.values.iter().enumerate() {
            action.set_parameter(i, value)?;
        }
        crate::boss_definition::validate_action(&action)?;
        Ok(action)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttackInput {
    pub name: String,
    pub actions: Vec<BossActionInput>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossInputs {
    /// Name, health, width, height, contact damage, pause seconds.
    pub fields: [String; 6],
    pub selection: AttackSelection,
    pub attacks: Vec<AttackInput>,
}

impl BossInputs {
    pub fn from_definition(boss: &BossDefinition) -> Self {
        Self {
            fields: [
                boss.name.clone(),
                boss.health.to_string(),
                boss.size.0.to_string(),
                boss.size.1.to_string(),
                boss.contact_damage.to_string(),
                boss.pause_seconds.to_string(),
            ],
            selection: boss.selection,
            attacks: boss
                .attacks
                .iter()
                .map(|a| AttackInput {
                    name: a.name.clone(),
                    actions: a.actions.iter().map(BossActionInput::from_action).collect(),
                })
                .collect(),
        }
    }

    pub fn build(&self) -> Result<BossDefinition, String> {
        let parse = |i: usize| crate::behavior::parse_nonnegative(self.fields[i].trim());
        let boss = BossDefinition {
            name: self.fields[0].trim().to_owned(),
            health: self.fields[1]
                .trim()
                .parse()
                .map_err(|_| "Health: expected integer 1..4294967295")?,
            size: (
                parse(2).map_err(|e| format!("Width: {e}"))?,
                parse(3).map_err(|e| format!("Height: {e}"))?,
            ),
            contact_damage: self.fields[4]
                .trim()
                .parse()
                .map_err(|_| "Contact damage: expected integer 0..255")?,
            pause_seconds: parse(5).map_err(|e| format!("Pause: {e}"))?,
            selection: self.selection,
            attacks: self
                .attacks
                .iter()
                .map(|a| {
                    Ok(AttackDefinition {
                        name: a.name.trim().to_owned(),
                        actions: a
                            .actions
                            .iter()
                            .enumerate()
                            .map(|(i, row)| {
                                row.build().map_err(|e| {
                                    format!("Attack {:?}, action {}: {e}", a.name, i + 1)
                                })
                            })
                            .collect::<Result<_, String>>()?,
                    })
                })
                .collect::<Result<_, String>>()?,
        };
        boss.validate()?;
        Ok(boss)
    }

    pub fn apply(&self, boss: &mut BossDefinition) -> Result<(), String> {
        *boss = self.build()?;
        Ok(())
    }

    pub fn add_attack(&mut self, name: &str) -> Result<usize, String> {
        self.check_name(name, None)?;
        self.attacks.push(AttackInput {
            name: name.trim().to_owned(),
            actions: Vec::new(),
        });
        Ok(self.attacks.len() - 1)
    }

    fn check_name(&self, name: &str, except: Option<usize>) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Attack name must not be empty".into());
        }
        if self
            .attacks
            .iter()
            .enumerate()
            .any(|(i, a)| Some(i) != except && a.name.trim() == name.trim())
        {
            return Err("Duplicate attack name".into());
        }
        Ok(())
    }

    pub fn rename_attack(&mut self, index: usize, name: &str) -> Result<(), String> {
        self.check_name(name, Some(index))?;
        self.attacks.get_mut(index).ok_or("missing attack")?.name = name.trim().to_owned();
        Ok(())
    }

    pub fn move_attack(&mut self, index: usize, up: bool) -> Result<(), String> {
        move_row(&mut self.attacks, index, up)
    }
    pub fn remove_attack(&mut self, index: usize) -> Result<(), String> {
        remove_row(&mut self.attacks, index)
    }
    pub fn add_action(&mut self, attack: usize, kind: usize) -> Result<(), String> {
        let row = BossActionInput::new(kind)?;
        self.attacks
            .get_mut(attack)
            .ok_or("missing attack")?
            .actions
            .push(row);
        Ok(())
    }
    pub fn move_action(&mut self, attack: usize, index: usize, up: bool) -> Result<(), String> {
        move_row(
            &mut self
                .attacks
                .get_mut(attack)
                .ok_or("missing attack")?
                .actions,
            index,
            up,
        )
    }
    pub fn remove_action(&mut self, attack: usize, index: usize) -> Result<(), String> {
        remove_row(
            &mut self
                .attacks
                .get_mut(attack)
                .ok_or("missing attack")?
                .actions,
            index,
        )
    }
}
