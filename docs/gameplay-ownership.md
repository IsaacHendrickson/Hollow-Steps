## Gameplay ownership

`GameState` retains all rooms; each `Room` owns its bounds, player-center
respawn point, items, exits, and enemies. Only active-room enemies update.
Enemies use center-to-center distance for detection, chase horizontally within
room walls, and return to their patrol interval when the player leaves range.
`Player` owns pure health/timer state; `GameState` resolves rectangular contact
and respawns at the room spawn with full health, cleared velocity, and a fresh
one-second grace period. Items and enemy positions persist through respawn.
Defeated enemies stay removed when revisiting a room. Room transitions and
respawns cancel active swings and pending jump assistance.
All timers use simulation delta time; rendering only reads state.

