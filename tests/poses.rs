use hollow_steps::body::{BodyInstance, BodyTemplate, BodyVisual};
use hollow_steps::pose::{load_poses, Pose};
use std::{path::Path, sync::Arc};

fn body() -> BodyInstance {
    let mut template: BodyTemplate =
        ron::from_str(include_str!("fixtures/bodies/motion.ron")).unwrap();
    template.pose_map = [
        ("head".into(), "root".into()),
        ("hand_l".into(), "absent".into()),
    ]
    .into();
    let visual: BodyVisual = ron::from_str(include_str!("fixtures/body_visual.ron")).unwrap();
    BodyInstance::new(
        visual.clone(),
        &[(visual.template.clone(), Arc::new(template))].into(),
    )
    .unwrap()
}

#[test]
fn semantic_resolution_ignores_missing_points() {
    let mut body = body();
    let pose: Pose = ron::from_str(include_str!("fixtures/poses/windup.ron")).unwrap();
    let targets = pose.resolve(&body.template);
    assert_eq!(targets.len(), 1);
    assert_eq!(
        targets["root"].x,
        body.template.points["root"].rest.0 - 0.20
    );
    pose.apply(&mut body);
    assert_eq!(body.targets["root"], targets["root"]);
    assert_eq!(
        load_poses(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("poses")
        )
        .unwrap()
        .len(),
        1
    );
}
