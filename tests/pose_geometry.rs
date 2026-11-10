//! Compile-time contracts: geometry APIs accept gameplay inputs, never body state.
use hollow_steps::behavior::BehaviorContext;
use hollow_steps::enemy::Enemy;
use hollow_steps::enemy_attack::{AttackBox, AttackRuntime, DamageHit};
use macroquad::math::Vec2;

#[test]
fn hitbox_and_attackbox_signatures_do_not_accept_body_state() {
    let _: fn(AttackBox, Vec2) -> Vec2 = AttackBox::center;
    let _: fn(AttackBox, Vec2, Vec2, Vec2) -> bool = AttackBox::overlaps;
    let _: fn(AttackBox) -> DamageHit = AttackBox::hit;
    let _: fn(&Enemy) -> Vec2 = Enemy::size;
    let _: fn(&Enemy, Vec2, Vec2) -> bool = Enemy::touches;
    let _: fn(&Enemy, Vec2, Vec2) -> Option<DamageHit> = Enemy::attack_hit;
    let _: fn(Vec2, BehaviorContext, f32, f32, AttackBox, f32, f32) -> AttackRuntime =
        AttackRuntime::lunge;
    type JumpConstructor =
        fn(Vec2, BehaviorContext, Vec2, f32, f32, Option<AttackBox>, f32, f32) -> AttackRuntime;
    let _: JumpConstructor = AttackRuntime::jump;
    let _: fn(Vec2, BehaviorContext, f32, AttackBox, f32, f32) -> AttackRuntime =
        AttackRuntime::slam;
    let _: fn(&mut AttackRuntime, &mut Vec2, BehaviorContext, f64) -> (f64, bool) =
        AttackRuntime::step;
}
