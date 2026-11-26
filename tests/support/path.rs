//! Path replay: a scripted list of inputs played on the real `Player`, on a room's own terrain,
//! hazards and exits (`docs/room-tools.md`). It proves what a room claims: that a path from an
//! arrival to an exit exists, and that a gate needs its ability or a locked exit its key.
//!
//! - `assert_reaches(room, arrival, script, target)`: the script gets there without being hit.
//! - `assert_blocked(room, arrival, script, target)`: the script does not get there.
//! - `assert_blocked_without(room, arrival, script, target, lack)`: the script gets there holding
//!   everything it holds, and does not get there without `lack` (an `Ability` or `Lack::Key`).
//!   Both halves are required: a script that never gets there proves nothing about the gate.
//!
//! Each has a `check_` form that returns the failure text instead of panicking. A failure prints
//! what was tried and a trace: a row every tenth tick (position, velocity, grounded, the nearest
//! terrain edge, any hazard touched) and the last tick.
#![allow(unused_imports)] // each test crate that includes `support` uses a part of this

mod replay;
mod script;
mod trace;

pub use replay::{replay, Arrival, Target, Verdict};
pub use script::{dash, jump, run_left, run_right, wait, Lack, PathScript};

use hollow_steps::editor::RoomLayout;

/// The tick the script reaches `target` at, or why not (with the trace).
pub fn check_reaches(
    room: &RoomLayout,
    arrival: Arrival,
    script: &PathScript,
    target: Target,
) -> Result<usize, String> {
    let played = replay(room, arrival, script);
    match played.verdict(room, target, &script.holds) {
        Verdict::Reached { tick } => Ok(tick),
        verdict => Err(trace::report(
            "the path does not reach its target",
            room,
            arrival,
            script,
            target,
            &played,
            verdict,
        )),
    }
}

/// Ok when the script does not reach `target`; the trace of where it got when it does.
pub fn check_blocked(
    room: &RoomLayout,
    arrival: Arrival,
    script: &PathScript,
    target: Target,
) -> Result<(), String> {
    let played = replay(room, arrival, script);
    match played.verdict(room, target, &script.holds) {
        Verdict::Reached { .. } => Err(trace::report(
            "the path reaches its target, but was meant to be blocked",
            room,
            arrival,
            script,
            target,
            &played,
            played.verdict(room, target, &script.holds),
        )),
        _ => Ok(()),
    }
}

/// Ok when the script reaches `target` as it is and does not once `lack` is taken away.
pub fn check_blocked_without(
    room: &RoomLayout,
    arrival: Arrival,
    script: &PathScript,
    target: Target,
    lack: impl Into<Lack>,
) -> Result<(), String> {
    let lack = lack.into();
    check_reaches(room, arrival, script, target).map_err(|trace| {
        format!("the script must reach its target holding everything, or it proves nothing about {lack}:\n{trace}")
    })?;
    check_blocked(room, arrival, &script.without(lack), target).map_err(|trace| {
        format!("{lack} is not needed: the same script gets there without it:\n{trace}")
    })
}

pub fn assert_reaches(room: &RoomLayout, arrival: Arrival, script: &PathScript, target: Target) {
    if let Err(message) = check_reaches(room, arrival, script, target) {
        panic!("{message}");
    }
}

pub fn assert_blocked(room: &RoomLayout, arrival: Arrival, script: &PathScript, target: Target) {
    if let Err(message) = check_blocked(room, arrival, script, target) {
        panic!("{message}");
    }
}

pub fn assert_blocked_without(
    room: &RoomLayout,
    arrival: Arrival,
    script: &PathScript,
    target: Target,
    lack: impl Into<Lack>,
) {
    if let Err(message) = check_blocked_without(room, arrival, script, target, lack) {
        panic!("{message}");
    }
}
