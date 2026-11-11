//! Value checks shared by the action registry, conditions and definitions.

pub(crate) fn positive(value: f32) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err("must be finite and positive".into())
    }
}
pub(super) fn boss_count(value: u16) -> Result<(), String> {
    if (1..=256).contains(&value) {
        Ok(())
    } else {
        Err("must be an integer in 1..256".into())
    }
}

pub(super) fn finite(value: f32) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err("expected a finite value".into())
    }
}

// Integer ranges are enforced by the loader and editor's typed parsing.
pub(super) fn valid_count(_: u16) -> Result<(), String> {
    Ok(())
}
pub(super) fn valid_damage(_: u8) -> Result<(), String> {
    Ok(())
}

pub(crate) fn nonnegative(value: f32) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!("expected a finite nonnegative value, got {value}"))
    }
}

pub(super) fn valid_pose_name(name: Option<String>) -> Result<(), String> {
    if let Some(name) = name {
        crate::pose::validate_name(&name)?;
    }
    Ok(())
}
