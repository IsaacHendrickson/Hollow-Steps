//! The room draft: the authored room being edited (enemies, boss, layout) and every edit of it
//! that checks itself on a copy, so a refused edit leaves the draft as it was. Split out of
//! `editor` (which re-exports `RoomDraft`) to keep that file small.
use macroquad::math::{Rect, Vec2};

use crate::editor::{ExitPlacement, ItemPlacement, Placement, RoomLayout, Selection};
use crate::enemy::EnemyDefinitions;
use crate::room::{EnemyPlacement, Room, RoomEdge, RoomEnemies, RoomId};

#[derive(Clone, Debug, PartialEq)]
pub struct RoomDraft {
    pub enemies: Vec<EnemyPlacement>,
    pub boss: Option<crate::room::BossPlacement>,
    pub layout: RoomLayout,
}

impl RoomDraft {
    pub fn new_room(template: &RoomLayout) -> Self {
        let mut layout = template.clone();
        layout.name = None; // named by whoever creates it, not by the template
        layout.terrain.clear(); // empty: only size, floor, spawn, background, look carry over
        layout.hazards.clear();
        layout.clear_puzzles();
        layout.items.clear();
        layout.charms.clear();
        layout.exits.clear();
        layout.gates.clear(); // marks on the template's exits mean nothing in an empty room
        layout.map_cell = None; // a copied cell would put the new room on top of its template
        layout.piece = None; // a mark and its slots belong to the template, not to a new room
        layout.slots.clear();
        Self {
            enemies: Vec::new(),
            boss: None,
            layout,
        }
    }

    pub fn from_document(document: RoomEnemies, fallback: &Room) -> Self {
        Self {
            enemies: document.enemies,
            boss: document.boss,
            layout: document
                .layout
                .unwrap_or_else(|| RoomLayout::from_room(fallback)),
        }
    }

    pub fn validate_local(&self, definitions: &EnemyDefinitions) -> Result<(), String> {
        self.validate_layout_and_variety()?;
        for enemy in &self.enemies {
            let size = definitions.size(&enemy.definition)?;
            if size.y > self.layout.floor_y - self.layout.bounds.1 {
                return Err("enemy is taller than the room".into());
            }
            definitions.spawn(
                &enemy.definition,
                enemy.patrol_left,
                enemy.patrol_right,
                self.layout.world_bounds(),
            )?;
        }
        Ok(())
    }

    pub fn validate(
        &self,
        definitions: &EnemyDefinitions,
        rooms: &[(RoomId, RoomLayout)],
    ) -> Result<(), String> {
        self.validate_local(definitions)?;
        for exit in &self.layout.exits {
            let target = rooms
                .iter()
                .find(|(id, _)| *id == exit.target_room)
                .ok_or_else(|| format!("exit targets missing room {:?}", exit.target_room))?;
            if !target.1.contains_body(exit.spawn, Vec2::new(32.0, 48.0)) {
                return Err(format!(
                    "exit spawn outside destination {:?}",
                    exit.target_room
                ));
            }
            target.1.validate_arrival(exit.spawn)?;
            // A door needs Up, so arriving in front of one is no immediate return trip.
            if target.1.exits.iter().any(|e| {
                !e.is_door()
                    && e.exit().zone(target.1.rect()).overlaps(&Rect::new(
                        exit.spawn.0 - 16.0,
                        exit.spawn.1 - 24.0,
                        32.0,
                        48.0,
                    ))
            }) {
                return Err("exit destination overlaps an exit (immediate return trip)".into());
            }
        }
        Ok(())
    }

    pub fn serialize(
        &self,
        definitions: &EnemyDefinitions,
        rooms: &[(RoomId, RoomLayout)],
    ) -> Result<String, String> {
        self.validate(definitions, rooms)?;
        ron::ser::to_string_pretty(
            &RoomEnemies {
                enemies: self.enemies.clone(),
                boss: self.boss.clone(),
                layout: Some(self.layout.clone()),
            },
            ron::ser::PrettyConfig::default(),
        )
        .map_err(|e| e.to_string())
    }

    pub fn place(
        &mut self,
        placement: Placement,
        position: Vec2,
        definitions: &EnemyDefinitions,
    ) -> Result<Selection, String> {
        if !position.is_finite() {
            return Err("position must be finite".into());
        }
        let mut next = self.clone();
        let selection = match placement {
            Placement::Enemy(definition) => {
                let index = next.enemies.len();
                next.enemies
                    .push(EnemyPlacement::new(definition, position.x, position.x));
                Selection::Enemy(index)
            }
            Placement::Item(item) => {
                let index = next.layout.items.len();
                next.layout.items.push(ItemPlacement {
                    item,
                    position: position.into(),
                });
                Selection::Item(index)
            }
            Placement::Charm(charm) => next.layout.place_charm(charm, position),
            Placement::Boss(_) => return Err("boss placement needs boss definitions".into()),
        };
        next.validate_local(definitions)?;
        *self = next;
        Ok(selection)
    }

    pub fn move_to(
        &mut self,
        selection: Selection,
        position: Vec2,
        definitions: &EnemyDefinitions,
    ) -> Result<(), String> {
        if !position.is_finite() {
            return Err("position must be finite".into());
        }
        let mut next = self.clone();
        match selection {
            Selection::Boss => return Err("boss movement needs boss definitions".into()),
            Selection::Enemy(i) => {
                let enemy = next.enemies.get_mut(i).ok_or("missing enemy")?;
                let delta = position.x - enemy.patrol_left;
                enemy.patrol_left += delta;
                enemy.patrol_right += delta;
            }
            Selection::PatrolLeft(i) => {
                next.enemies.get_mut(i).ok_or("missing enemy")?.patrol_left = position.x;
            }
            Selection::PatrolRight(i) => {
                next.enemies.get_mut(i).ok_or("missing enemy")?.patrol_right = position.x;
            }
            Selection::Item(i) => {
                next.layout.items.get_mut(i).ok_or("missing item")?.position = position.into()
            }
            Selection::Charm(i) => next.layout.move_charm(i, position)?,
            Selection::Hazard(i) => next.layout.move_hazard(i, position)?,
            Selection::Breakable(_) | Selection::Switch(_) | Selection::Barrier(_) => {
                next.layout.move_puzzle(selection, position)?
            }
            Selection::Spawn => next.layout.spawn = position.into(),
            Selection::Exit(i) => {
                let exit = next.layout.exits.get_mut(i).ok_or("missing exit")?;
                if exit.is_door() {
                    exit.move_door_to(position);
                } else {
                    exit.offset = match exit.edge {
                        RoomEdge::Left | RoomEdge::Right => {
                            position.y - next.layout.bounds.1 - exit.length / 2.0
                        }
                        _ => position.x - next.layout.bounds.0 - exit.length / 2.0,
                    };
                }
            }
            Selection::Destination(i) => {
                next.layout.exits.get_mut(i).ok_or("missing exit")?.spawn = position.into()
            }
            // These change another room or a link under way: `EditorSession::move_selection`.
            Selection::Inbound { .. } | Selection::LinkArrival(_) => {
                return Err("that arrival is not part of this room".into())
            }
        }
        next.layout.follow_exits(&self.layout.exits); // gates follow a dragged exit
        next.validate_local(definitions)?;
        *self = next;
        Ok(())
    }

    pub fn add_exit(&mut self, edge: RoomEdge, target_room: RoomId, spawn: Vec2) -> Selection {
        let length = match edge {
            RoomEdge::Left | RoomEdge::Right => self.layout.bounds.3,
            _ => self.layout.bounds.2,
        }
        .min(96.0);
        let index = self.layout.exits.len();
        self.layout.exits.push(ExitPlacement::edge_exit(
            edge,
            0.0,
            length,
            target_room,
            spawn.into(),
        ));
        Selection::Exit(index)
    }

    pub fn set_exit_target(
        &mut self,
        index: usize,
        target_room: RoomId,
        spawn: Vec2,
    ) -> Result<(), String> {
        if !spawn.is_finite() {
            return Err("destination must be finite".into());
        }
        let exit = self.layout.exits.get_mut(index).ok_or("missing exit")?;
        exit.target_room = target_room;
        exit.spawn = spawn.into();
        Ok(())
    }

    pub fn delete(&mut self, selection: Selection) -> Result<(), String> {
        match selection {
            Selection::Boss if self.boss.is_some() => self.boss = None,
            Selection::Enemy(i) if i < self.enemies.len() => {
                self.enemies.remove(i);
            }
            Selection::Item(i) if i < self.layout.items.len() => {
                self.layout.items.remove(i);
            }
            Selection::Charm(i) if i < self.layout.charms.len() => {
                self.layout.charms.remove(i);
            }
            Selection::Hazard(i) if i < self.layout.hazards.len() => {
                self.layout.hazards.remove(i);
            }
            s if s.is_puzzle() => self.layout.delete_puzzle(s)?,
            Selection::Exit(i) | Selection::Destination(i) if i < self.layout.exits.len() => {
                self.layout.exits.remove(i);
                self.layout.drop_gates_without_exit();
            }
            _ => return Err("select a boss, enemy, item, charm, hazard, or exit to delete".into()),
        }
        Ok(())
    }

    pub fn warnings(&self) -> Vec<&'static str> {
        if self.layout.exits.is_empty() {
            vec!["Warning: room has no exits"]
        } else {
            Vec::new()
        }
    }

    pub fn validate_boss(
        &self,
        definitions: &crate::boss_definition::BossDefinitions,
    ) -> Result<(), String> {
        self.layout.validate_geometry()?;
        if let Some(placement) = &self.boss {
            let size = definitions.size(&placement.definition)?;
            if !self.layout.contains_body(placement.position, size) {
                return Err(format!(
                    "boss {:?}: position/body must fit within room",
                    placement.definition
                ));
            }
        }
        Ok(())
    }

    pub fn place_boss(
        &mut self,
        name: &str,
        position: Vec2,
        definitions: &crate::boss_definition::BossDefinitions,
    ) -> Result<Selection, String> {
        if self.boss.is_some() {
            return Err("room already has a boss; delete it before placing another".into());
        }
        let mut next = self.clone();
        next.boss = Some(crate::room::BossPlacement::new(name, position.into()));
        next.validate_boss(definitions)?;
        *self = next;
        Ok(Selection::Boss)
    }

    pub fn move_boss(
        &mut self,
        position: Vec2,
        definitions: &crate::boss_definition::BossDefinitions,
    ) -> Result<(), String> {
        let mut next = self.clone();
        next.boss.as_mut().ok_or("missing boss")?.position = position.into();
        next.validate_boss(definitions)?;
        *self = next;
        Ok(())
    }

    pub fn build(&self, id: RoomId, definitions: &EnemyDefinitions) -> Result<Room, String> {
        self.validate_local(definitions)?;
        let mut room = Room {
            id,
            name: None,
            bounds: self.layout.rect(),
            floor_y: self.layout.floor_y,
            terrain: Default::default(),
            hazards: Vec::new(),
            puzzles: Default::default(),
            terrain_style: Default::default(),
            background: Vec::new(),
            gates: Vec::new(),
            map_cell: None,
            piece: None,
            slots: Vec::new(),
            spawn: self.layout.spawn.into(),
            enemies: self
                .enemies
                .iter()
                .map(|p| {
                    definitions
                        .spawn(
                            &p.definition,
                            p.patrol_left,
                            p.patrol_right,
                            self.layout.world_bounds(),
                        )
                        .map(|enemy| enemy.with_placement(p))
                })
                .collect::<Result<_, _>>()?,
            exits: Vec::new(),
            items: Vec::new(),
            charms: Vec::new(),
            coins: Vec::new(),
            boss_special: None,
        };
        self.layout.apply(&mut room)?;
        Ok(room)
    }
}
