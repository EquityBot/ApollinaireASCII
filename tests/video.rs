// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

use apollinaire_ascii::{
    Animation, MAX_FRAMES, RenderOptions, TimedFrame,
    video::{capture_due, check_capture, frame_durations, render_rgba, sample_size, sample_times},
};

#[test]
fn sampling_covers_the_clip_at_the_chosen_rate() {
    let times = sample_times(10_000.0, 1_000.0, 2_000.0, 10).unwrap();
    assert_eq!(times.len(), 10);
    assert_eq!(times[0], 1_000.0);
    assert_eq!(times[9], 1_900.0);
    // 12 fps does not divide a second evenly; rounding keeps the total exact.
    let times = sample_times(1_000.0, 0.0, 1_000.0, 12).unwrap();
    assert_eq!(times.len(), 12);
    let durations = frame_durations(&times, 1_000.0).unwrap();
    assert_eq!(durations.iter().sum::<u32>(), 1_000);
    assert!(durations.iter().all(|d| (83..=84).contains(d)));
}

#[test]
fn a_final_sample_shorter_than_ten_ms_is_omitted() {
    let times = sample_times(2_000.0, 0.0, 1_005.0, 10).unwrap();
    assert_eq!(times.len(), 10);
    let durations = frame_durations(&times, 1_005.0).unwrap();
    assert_eq!(durations.last(), Some(&105));
    assert_eq!(durations.iter().sum::<u32>(), 1_005);
    let times = sample_times(2_000.0, 0.0, 1_010.0, 10).unwrap();
    assert_eq!(times.len(), 11);
    assert_eq!(frame_durations(&times, 1_010.0).unwrap().last(), Some(&10));
}

#[test]
fn sampling_rejects_bad_ranges_rates_and_frame_counts() {
    for (duration, start, end, fps) in [
        (f64::INFINITY, 0.0, 1_000.0, 10),
        (f64::NAN, 0.0, 1_000.0, 10),
        (0.0, 0.0, 0.0, 10),
        (5_000.0, -1.0, 1_000.0, 10),
        (5_000.0, 0.0, 5_001.0, 10),
        (5_000.0, 2_000.0, 1_000.0, 10),
        (5_000.0, 1_000.0, 1_005.0, 10),
        (5_000.0, f64::NAN, 1_000.0, 10),
        (5_000.0, 0.0, 1_000.0, 0),
        (5_000.0, 0.0, 1_000.0, 61),
    ] {
        assert!(sample_times(duration, start, end, fps).is_err());
    }
    let limit = MAX_FRAMES as f64 * 100.0;
    assert_eq!(
        sample_times(limit, 0.0, limit, 10).unwrap().len(),
        MAX_FRAMES
    );
    let error = sample_times(limit + 100.0, 0.0, limit + 100.0, 10).unwrap_err();
    assert!(error.contains("601 frames"), "{error}");
    // A final sample under 10 ms is dropped before the frame limit applies.
    let times = sample_times(10_001.0, 0.0, 10_001.0, 60).unwrap();
    assert_eq!(times.len(), MAX_FRAMES);
    assert_eq!(
        frame_durations(&times, 10_001.0)
            .unwrap()
            .iter()
            .sum::<u32>(),
        10_001
    );
}

#[test]
fn camera_captures_keep_durations_valid() {
    assert!(capture_due(None, 0.0, 1_000.0));
    assert!(!capture_due(None, 995.0, 1_000.0));
    assert!(!capture_due(Some(100.0), 104.0, 1_000.0));
    assert!(capture_due(Some(100.0), 110.0, 1_000.0));
    // Present day, present time: irregular timer ticks become durations that sum to the recording.
    let mut times = vec![];
    for now in [0.0, 30.2, 34.0, 95.7, 140.0, 196.0] {
        if capture_due(times.last().copied(), now, 200.0) {
            times.push(now);
        }
    }
    assert_eq!(times, [0.0, 30.2, 95.7, 140.0]);
    let durations = frame_durations(&times, 200.0).unwrap();
    assert_eq!(durations, [30, 66, 44, 60]);
    let frames = durations
        .iter()
        .map(|&duration_ms| TimedFrame {
            frame: apollinaire_ascii::Frame::from_text("#").unwrap(),
            duration_ms,
        })
        .collect();
    assert_eq!(Animation::new(frames, true).unwrap().duration_ms(), 200);
}

#[test]
fn durations_reject_empty_close_or_long_gaps() {
    assert!(frame_durations(&[], 100.0).is_err());
    assert!(frame_durations(&[0.0, 5.0], 100.0).is_err());
    assert!(frame_durations(&[0.0], 5.0).is_err());
    assert!(frame_durations(&[0.0], 60_001.0).is_err());
    assert!(frame_durations(&[0.0, f64::NAN], 100.0).is_err());
    assert_eq!(frame_durations(&[0.0], 60_000.0).unwrap(), [60_000]);
}

#[test]
fn sample_size_keeps_aspect_and_media_limits() {
    assert_eq!(sample_size(1920, 1080, 80).unwrap(), (160, 90));
    assert_eq!(sample_size(640, 480, 512).unwrap(), (640, 480));
    assert_eq!(sample_size(4096, 1, 512).unwrap(), (1024, 1));
    assert!(sample_size(0, 480, 80).is_err());
    assert!(sample_size(4097, 480, 80).is_err());
    assert!(sample_size(640, 480, 0).is_err());
    assert!(sample_size(640, 480, 513).is_err());
}

#[test]
fn capture_budget_matches_project_limits() {
    assert!(check_capture(80, 24, 600).is_ok());
    assert!(check_capture(80, 24, 601).is_err());
    assert!(check_capture(80, 24, 0).is_err());
    assert!(check_capture(512, 256, 61).is_ok());
    assert!(check_capture(512, 256, 62).unwrap_err().contains("cell"));
}

#[test]
fn rgba_pixels_render_with_the_image_converter() {
    let options = RenderOptions {
        width: 2,
        height: Some(1),
        ..RenderOptions::default()
    };
    let frame = render_rgba(2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255], &options).unwrap();
    assert_eq!(frame.text(), " @");
    assert!(render_rgba(2, 1, vec![0; 4], &options).is_err());
    let height = RenderOptions {
        width: 80,
        ..RenderOptions::default()
    }
    .output_height(160, 90);
    assert_eq!(height, 23);
}
