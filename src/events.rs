//! Gameplay events: a plain record of what just happened, in the order it happened.
//!
//! Simulation code pushes an event at the moment the thing happens, exactly once.
//! `GameState::take_events` hands the queue to whoever reacts to events (hit-stop and
//! screen shake today, particles and sound later), so those never need to poke at the
//! rules to find out what occurred.

/// Something that happened during a simulation tick.
///
/// `EnemyHit` also covers the arena boss (its death is `BossDefeated`, not `EnemyDied`);
/// `RoomEntered` is raised only when the player walks through an exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Events in the order they happened, until someone takes them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventQueue {
    events: Vec<GameEvent>,
}

impl EventQueue {
    pub fn push(&mut self, event: GameEvent) {
        self.events.push(event);
    }

    /// Adds `events` to the end of the queue, keeping their order.
    pub fn extend(&mut self, events: impl IntoIterator<Item = GameEvent>) {
        self.events.extend(events);
    }

    /// Removes and returns all events, oldest first.
    pub fn take(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
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
