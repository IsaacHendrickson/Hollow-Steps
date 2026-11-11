//! Gameplay events: a plain record of what just happened, in the order it happened.
//!
//! Simulation code pushes an event at the moment the thing happens, exactly once.
//! `GameState::take_events` hands the queue to whoever reacts to events (hit-stop and
//! screen shake, particles and sound), so those never need to poke at the rules to find
//! out what occurred. `take_located_events` also says where in the world each one happened.

use macroquad::math::Vec2;
use serde::Deserialize;

/// Something that happened during a simulation tick.
///
/// `EnemyHit` also covers the arena boss (its death is `BossDefeated`, not `EnemyDied`);
/// `RoomEntered` is raised only when the player walks through an exit.
///
/// Event names are what the data files (`particles.ron`, `audio.ron`) use to pick effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
pub enum GameEvent {
    /// A jump started (from the floor, coyote time or a buffered press).
    PlayerJumped,
    /// The player touched down after falling.
    PlayerLanded,
    /// A dash started.
    PlayerDashed,
    /// A melee swing started, whether or not it hits anything.
    PlayerAttacked,
    /// A swing registered a hit on an enemy or the boss (once per target per swing).
    EnemyHit,
    /// An enemy's health reached zero (once per enemy).
    EnemyDied,
    /// The player lost health.
    PlayerDamaged,
    /// A coin was collected.
    CoinCollected,
    /// A world item was picked up.
    ItemPickedUp,
    /// The player went through an exit into another room.
    RoomEntered,
    /// The arena boss's health reached zero.
    BossDefeated,
}

/// An event and, when it has one, the world position it happened at.
///
/// The position is where the effect belongs, taken at the end of the tick the event happened
/// in (so at most one tick of movement from the exact moment): the player's feet for a jump,
/// landing, dash or swing, the player's center for damage, and the center of the enemy or
/// boss for a hit or a defeat. Events with no useful place (a coin, an item, a room change)
/// carry none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocatedEvent {
    pub event: GameEvent,
    pub at: Option<Vec2>,
}

/// Events in the order they happened, until someone takes them.
///
/// `events` and `points` always have the same length: `points[i]` is where `events[i]`
/// happened, if known.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventQueue {
    events: Vec<GameEvent>,
    points: Vec<Option<Vec2>>,
}

impl EventQueue {
    pub fn push(&mut self, event: GameEvent) {
        self.events.push(event);
        self.points.push(None);
    }

    /// Adds an event that happened at `at`.
    pub fn push_at(&mut self, event: GameEvent, at: Vec2) {
        self.events.push(event);
        self.points.push(Some(at));
    }

    /// Adds `events` to the end of the queue, keeping their order.
    pub fn extend(&mut self, events: impl IntoIterator<Item = GameEvent>) {
        for event in events {
            self.push(event);
        }
    }

    /// Like `extend`, for events that all happened at `at`.
    pub fn extend_at(&mut self, events: impl IntoIterator<Item = GameEvent>, at: Vec2) {
        for event in events {
            self.push_at(event, at);
        }
    }

    /// Removes and returns all events, oldest first.
    pub fn take(&mut self) -> Vec<GameEvent> {
        self.points.clear();
        std::mem::take(&mut self.events)
    }

    /// Removes and returns all events with their positions, oldest first.
    pub fn take_located(&mut self) -> Vec<LocatedEvent> {
        let events = std::mem::take(&mut self.events);
        let points = std::mem::take(&mut self.points);
        events
            .into_iter()
            .zip(points)
            .map(|(event, at)| LocatedEvent { event, at })
            .collect()
    }

    pub fn as_slice(&self) -> &[GameEvent] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn located_events_keep_their_order_and_positions() {
        let mut queue = EventQueue::default();
        queue.push(GameEvent::CoinCollected);
        queue.push_at(GameEvent::EnemyHit, Vec2::new(10.0, 20.0));
        queue.extend_at(
            [GameEvent::PlayerJumped, GameEvent::PlayerAttacked],
            Vec2::ONE,
        );
        queue.extend([GameEvent::RoomEntered]);
        assert_eq!(
            queue.as_slice(),
            [
                GameEvent::CoinCollected,
                GameEvent::EnemyHit,
                GameEvent::PlayerJumped,
                GameEvent::PlayerAttacked,
                GameEvent::RoomEntered
            ]
        );
        let located = queue.take_located();
        let points: Vec<_> = located.iter().map(|located| located.at).collect();
        assert_eq!(
            points,
            [
                None,
                Some(Vec2::new(10.0, 20.0)),
                Some(Vec2::ONE),
                Some(Vec2::ONE),
                None
            ]
        );
        assert_eq!(located[1].event, GameEvent::EnemyHit);
        assert!(queue.is_empty());
        assert!(queue.take_located().is_empty());
    }

    #[test]
    fn taking_plain_events_also_forgets_their_positions() {
        let mut queue = EventQueue::default();
        queue.push_at(GameEvent::EnemyDied, Vec2::new(5.0, 5.0));
        assert_eq!(queue.take(), vec![GameEvent::EnemyDied]);
        // A later event must not inherit the dropped event's position.
        queue.push(GameEvent::CoinCollected);
        assert_eq!(
            queue.take_located(),
            vec![LocatedEvent {
                event: GameEvent::CoinCollected,
                at: None
            }]
        );
    }

    #[test]
    fn event_names_parse_from_ron_and_unknown_names_are_rejected() {
        assert_eq!(
            ron::from_str::<Vec<GameEvent>>("[EnemyHit, BossDefeated]").unwrap(),
            vec![GameEvent::EnemyHit, GameEvent::BossDefeated]
        );
        assert!(ron::from_str::<GameEvent>("EnemyHitt").is_err());
    }

    #[test]
    fn take_returns_events_in_order_and_empties_the_queue() {
        let mut queue = EventQueue::default();
        queue.push(GameEvent::PlayerJumped);
        queue.push(GameEvent::PlayerLanded);
        assert_eq!(
            queue.as_slice(),
            [GameEvent::PlayerJumped, GameEvent::PlayerLanded]
        );
        assert_eq!(
            queue.take(),
            vec![GameEvent::PlayerJumped, GameEvent::PlayerLanded]
        );
        assert!(queue.is_empty());
        assert!(queue.take().is_empty());
    }

    #[test]
    fn extend_keeps_order_after_what_is_already_queued() {
        let mut queue = EventQueue::default();
        queue.push(GameEvent::PlayerAttacked);
        queue.extend([GameEvent::EnemyHit, GameEvent::EnemyDied]);
        assert_eq!(
            queue.as_slice(),
            [
                GameEvent::PlayerAttacked,
                GameEvent::EnemyHit,
                GameEvent::EnemyDied
            ]
        );
    }
}
