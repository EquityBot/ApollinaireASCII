use apollinaire_ascii::{Animation, Frame, MAX_FRAMES, TimedFrame};

fn project() -> Animation {
    // A small Croft Manor storyboard, with distinct timings to follow every frame.
    Animation::new(
        "ABCDE"
            .chars()
            .enumerate()
            .map(|(i, c)| TimedFrame {
                frame: Frame::from_text(&c.to_string()).unwrap(),
                duration_ms: (i as u32 + 1) * 100,
            })
            .collect(),
        true,
    )
    .unwrap()
}

fn labels(project: &Animation) -> String {
    project.frames().iter().map(|f| f.frame.text()).collect()
}

#[test]
fn duplicate_range_preserves_order_timings_and_looping() {
    let mut p = project();
    p.duplicate_frames(1..3).unwrap();
    assert_eq!(labels(&p), "ABCBCDE");
    assert_eq!(
        p.frames().iter().map(|f| f.duration_ms).collect::<Vec<_>>(),
        [100, 200, 300, 200, 300, 400, 500]
    );
    assert!(p.looping());
    let mut p = project();
    p.duplicate_frames(0..5).unwrap();
    assert_eq!(labels(&p), "ABCDEABCDE");
    let mut p = project();
    p.duplicate_frames(4..5).unwrap();
    assert_eq!(labels(&p), "ABCDEE");
}

#[test]
fn delete_range_keeps_survivors_in_order_and_at_least_one_frame() {
    let mut p = project();
    p.remove_frames(1..3).unwrap();
    assert_eq!(labels(&p), "ADE");
    let before = p.clone();
    assert!(p.remove_frames(0..3).is_err());
    assert_eq!(p, before);
    p.remove_frames(1..3).unwrap();
    assert_eq!(labels(&p), "A");
    assert!(p.remove_frames(0..1).is_err());
}

#[test]
fn move_range_uses_destination_in_the_result_and_preserves_order() {
    let mut p = project();
    p.move_frames(1..3, 0).unwrap();
    assert_eq!(labels(&p), "BCADE");
    p.move_frames(0..2, 1).unwrap();
    assert_eq!(labels(&p), "ABCDE");
    p.move_frames(1..3, 2).unwrap();
    assert_eq!(labels(&p), "ADBCE");
    p.move_frames(2..4, 3).unwrap();
    assert_eq!(labels(&p), "ADEBC");
    p.move_frames(3..5, 0).unwrap();
    assert_eq!(labels(&p), "BCADE");
    let before = p.clone();
    p.move_frames(0..5, 0).unwrap();
    assert_eq!(p, before);
}

#[test]
fn duration_range_changes_only_its_members_and_validates_atomically() {
    let mut p = project();
    p.set_frame_durations(1..4, 10).unwrap();
    assert_eq!(labels(&p), "ABCDE");
    assert_eq!(
        p.frames().iter().map(|f| f.duration_ms).collect::<Vec<_>>(),
        [100, 10, 10, 10, 500]
    );
    p.set_frame_durations(1..4, 60_000).unwrap();
    for duration in [0, 9, 60_001, u32::MAX] {
        let before = p.clone();
        assert!(p.set_frame_durations(1..4, duration).is_err());
        assert_eq!(p, before);
    }
    let entry = project().frames()[0].clone();
    let mut p = Animation::new(vec![entry; 61], false).unwrap();
    let before = p.clone();
    assert!(p.set_frame_durations(0..61, 60_000).is_err());
    assert_eq!(p, before);
}

#[test]
fn invalid_ranges_and_destinations_leave_the_project_unchanged() {
    for (start, end) in [(0, 0), (2, 2), (3, 1), (0, 6), (5, 6), (0, usize::MAX)] {
        let range = start..end;
        let mut p = project();
        let before = p.clone();
        assert!(p.duplicate_frames(range.clone()).is_err());
        assert_eq!(p, before);
        assert!(p.remove_frames(range.clone()).is_err());
        assert_eq!(p, before);
        assert!(p.move_frames(range.clone(), 0).is_err());
        assert_eq!(p, before);
        assert!(p.set_frame_durations(range, 100).is_err());
        assert_eq!(p, before);
    }
    for destination in [4, 5, usize::MAX] {
        let mut p = project();
        let before = p.clone();
        assert!(p.move_frames(1..3, destination).is_err());
        assert_eq!(p, before);
    }
}

#[test]
fn duplicating_over_any_project_budget_fails_atomically() {
    for (frame, count, duration) in [
        (Frame::from_text("A").unwrap(), MAX_FRAMES, 10),
        (Frame::blank(512, 256).unwrap(), 61, 10),
        (Frame::from_text("A").unwrap(), 60, 60_000),
    ] {
        let mut p = Animation::new(
            vec![
                TimedFrame {
                    frame,
                    duration_ms: duration
                };
                count
            ],
            false,
        )
        .unwrap();
        let before = p.clone();
        assert!(p.duplicate_frames(0..1).is_err());
        assert_eq!(p, before);
    }
}
