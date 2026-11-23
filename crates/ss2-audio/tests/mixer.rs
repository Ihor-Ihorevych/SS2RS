//! Tests of the mixer core (roadmap T06-006, T06-013, T06-017), with analytic expectations: the samples a mixer must produce can be
//! computed by hand for constant and sine sources.

use ss2_audio::wav;
use ss2_audio::{
    AudioTolerance, Ducking, Mixer, MixerConfig, SampleBuffer, compare, render_offline,
};
use ss2_audio_api::{Bus, Listener, PlayParams, Priority, SoundId, Spatial, VoiceId, VoiceUpdate};
use ss2_core::math::Vec3;

const RATE: u32 = 48_000;
/// Equal-power centre: cos(45 degrees) on each side.
const CENTRE: f32 = std::f32::consts::FRAC_1_SQRT_2;

fn id(n: u32) -> SoundId {
    SoundId::from_raw(n).unwrap()
}

fn constant(value: f32, frames: usize) -> SampleBuffer {
    SampleBuffer::new(RATE, 1, vec![value; frames]).unwrap()
}

fn mixer_with(max_voices: usize) -> Mixer {
    Mixer::new(MixerConfig {
        max_voices,
        ..MixerConfig::default()
    })
    .unwrap()
}

fn render(m: &mut Mixer, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames * 2];
    m.render(&mut out);
    out
}

fn at(position: Vec3) -> PlayParams {
    PlayParams {
        spatial: Some(Spatial {
            position,
            velocity: Vec3::ZERO,
            min_distance: 2.0,
            max_distance: 10.0,
        }),
        ..PlayParams::sfx()
    }
}

fn near(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

// ---- one voice ------------------------------------------------------------------------------

#[test]
fn a_2d_voice_is_centred_with_equal_power_and_fades_in_over_its_first_block() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(0.5, 48_000));
    m.play(id(1), &PlayParams::sfx()).unwrap();
    let first = render(&mut m, 480);
    assert!(
        first[0].abs() < 0.01,
        "the first sample starts near zero (no click): {}",
        first[0]
    );
    assert!(
        near(first[2 * 479], 0.5 * CENTRE, 1e-3),
        "and has reached its gain by the end of the block: {}",
        first[2 * 479]
    );
    let second = render(&mut m, 480);
    assert!(
        second.iter().all(|s| near(*s, 0.5 * CENTRE, 1e-5)),
        "steady afterwards: {} {}",
        second[0],
        second[1]
    );
    assert!(near(second[0], second[1], 0.0), "both channels equal");
}

#[test]
fn a_sine_comes_out_as_the_sine_scaled_by_the_centre_gain() {
    let frames = 4_800;
    let sine: Vec<f32> = (0..frames)
        .map(|n| 0.8 * (2.0 * std::f32::consts::PI * 1000.0 * n as f32 / RATE as f32).sin())
        .collect();
    let mut m = mixer_with(8);
    m.add_sound(id(1), SampleBuffer::new(RATE, 1, sine.clone()).unwrap());
    m.play(id(1), &PlayParams::sfx()).unwrap();
    let out = render_offline(&mut m, frames, 480);
    // After the first block (the fade-in), the left channel is the source times the centre gain.
    let expected: Vec<f32> = sine[480..].iter().map(|s| s * CENTRE).collect();
    let got: Vec<f32> = out[2 * 480..].chunks(2).map(|f| f[0]).collect();
    let diff = compare(&got, &expected).unwrap();
    assert!(AudioTolerance::default().accepts(&diff), "{diff:?}");
}

#[test]
fn pitch_resamples_by_linear_interpolation() {
    let ramp: Vec<f32> = (0..4_000).map(|i| i as f32 / 4_000.0).collect();
    let mut m = mixer_with(8);
    m.add_sound(id(1), SampleBuffer::new(RATE, 1, ramp.clone()).unwrap());
    m.play(id(1), &PlayParams::sfx().with_pitch(2.0)).unwrap();
    render(&mut m, 100); // the fade-in block
    let out = render(&mut m, 100);
    for k in 0..100 {
        let want = ramp[2 * (100 + k)] * CENTRE;
        assert!(
            near(out[2 * k], want, 1e-4),
            "frame {k}: {} vs {want}",
            out[2 * k]
        );
    }
    // A half-speed voice interpolates between samples: frame n reads source position n / 2.
    let ramp: Vec<f32> = (0..400).map(|i| i as f32 / 400.0).collect(); // 0.0025 per frame: half a step is far above the tolerance
    let mut slow = mixer_with(8);
    slow.add_sound(id(1), SampleBuffer::new(RATE, 1, ramp.clone()).unwrap());
    slow.play(id(1), &PlayParams::sfx().with_pitch(0.5))
        .unwrap();
    render(&mut slow, 100);
    let out = render(&mut slow, 100);
    for k in [0usize, 1, 2, 3, 50, 99] {
        let position = (100 + k) as f32 * 0.5;
        let want = (ramp[position.floor() as usize] * (1.0 - position.fract())
            + ramp[position.floor() as usize + 1] * position.fract())
            * CENTRE;
        assert!(
            near(out[2 * k], want, 1e-4),
            "frame {k}: {} vs {want}",
            out[2 * k]
        );
    }
}

#[test]
fn a_source_at_another_rate_plays_at_the_right_speed() {
    let mut m = mixer_with(8);
    // 24 kHz: each source frame lasts two output frames.
    let ramp: Vec<f32> = (0..1_000).map(|i| i as f32 / 1_000.0).collect();
    m.add_sound(id(1), SampleBuffer::new(24_000, 1, ramp.clone()).unwrap());
    let v = m.play(id(1), &PlayParams::sfx()).unwrap();
    render(&mut m, 200);
    assert!(
        near(m.voice_state(v).unwrap().position as f32, 100.0, 1e-3),
        "{:?}",
        m.voice_state(v)
    );
}

#[test]
fn a_stereo_source_keeps_its_channels() {
    let mut m = mixer_with(8);
    let samples: Vec<f32> = (0..4_800).flat_map(|_| [1.0, 0.0]).collect(); // left only
    m.add_sound(id(1), SampleBuffer::new(RATE, 2, samples).unwrap());
    m.play(id(1), &PlayParams::sfx()).unwrap();
    render(&mut m, 480);
    let out = render(&mut m, 480);
    assert!(near(out[0], CENTRE, 1e-5) && near(out[1], 0.0, 1e-6));
}

#[test]
fn a_one_shot_ends_and_frees_its_voice_and_a_loop_goes_on() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(0.5, 100));
    let one = m.play(id(1), &PlayParams::sfx()).unwrap();
    let looped = m.play(id(1), &PlayParams::sfx().looped()).unwrap();
    let out = render(&mut m, 250);
    assert!(!m.is_playing(one), "the one-shot is over");
    assert!(m.is_playing(looped), "the loop is not");
    assert!(
        out[2 * 240].abs() > 0.1,
        "the loop still sounds after the source ended: {}",
        out[2 * 240]
    );
    assert_eq!(m.voice_count(), 1);
    let again = render(&mut m, 1000);
    assert!(
        again.iter().all(|s| s.abs() > 0.1),
        "a loop never goes silent"
    );
    assert!(
        m.voice_state(looped).unwrap().position < 100.0,
        "the cursor wraps"
    );
}

#[test]
fn a_loop_wraps_by_the_overshoot_so_a_fractional_step_does_not_drift() {
    let mut m = mixer_with(8);
    let ramp: Vec<f32> = (0..97).map(|i| i as f32 / 97.0).collect();
    m.add_sound(id(1), SampleBuffer::new(RATE, 1, ramp).unwrap());
    let v = m
        .play(id(1), &PlayParams::sfx().looped().with_pitch(1.37))
        .unwrap();
    render(&mut m, 1_000);
    let expected = (1_000.0f64 * 1.37_f32 as f64) % 97.0;
    let got = m.voice_state(v).unwrap().position;
    assert!((got - expected).abs() < 0.01, "{got} vs {expected}");
}

#[test]
fn a_loop_whose_period_is_not_a_whole_number_of_frames_stays_in_range() {
    let mut m = mixer_with(8);
    let ramp: Vec<f32> = (0..97).map(|i| i as f32 / 97.0).collect();
    m.add_sound(id(1), SampleBuffer::new(RATE, 1, ramp).unwrap());
    let v = m
        .play(id(1), &PlayParams::sfx().looped().with_pitch(1.37))
        .unwrap();
    for _ in 0..50 {
        render(&mut m, 333);
        let p = m.voice_state(v).unwrap().position;
        assert!((0.0..97.0).contains(&p), "{p}");
    }
}

// ---- space ----------------------------------------------------------------------------------

#[test]
fn a_voice_is_panned_by_where_it_is_relative_to_the_listener() {
    let play_at = |position: Vec3| {
        let mut m = mixer_with(8);
        m.add_sound(id(1), constant(1.0, 48_000));
        m.play(id(1), &at(position)).unwrap();
        render(&mut m, 480);
        let out = render(&mut m, 480);
        (out[0], out[1])
    };
    let (l, r) = play_at(Vec3::new(1.5, 0.0, 0.0)); // to the right, inside the minimum distance
    assert!(near(l, 0.0, 1e-4) && near(r, 1.0, 1e-4), "right: {l} {r}");
    let (l, r) = play_at(Vec3::new(-1.5, 0.0, 0.0));
    assert!(near(l, 1.0, 1e-4) && near(r, 0.0, 1e-4), "left: {l} {r}");
    let (l, r) = play_at(Vec3::new(0.0, 0.0, -1.5)); // straight ahead
    assert!(
        near(l, CENTRE, 1e-4) && near(r, CENTRE, 1e-4),
        "ahead: {l} {r}"
    );
    let (l, r) = play_at(Vec3::new(0.0, 0.0, 1.5)); // behind: no front-back cue, centred
    assert!(
        near(l, CENTRE, 1e-4) && near(r, CENTRE, 1e-4),
        "behind: {l} {r}"
    );
    let (l, r) = play_at(Vec3::ZERO); // on the listener
    assert!(
        near(l, CENTRE, 1e-4) && near(r, CENTRE, 1e-4),
        "inside the head: {l} {r}"
    );
}

#[test]
fn the_listener_turns_the_sound_field_and_a_bad_listener_is_refused() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 48_000));
    m.play(id(1), &at(Vec3::new(0.0, 0.0, -1.5))).unwrap(); // ahead of the default listener
    // Turn the listener to face +X: the sound is now to its left.
    m.set_listener(&Listener {
        forward: Vec3::X,
        up: Vec3::Y,
        ..Listener::default()
    })
    .unwrap();
    render(&mut m, 480);
    let out = render(&mut m, 480);
    assert!(out[0] > 0.99 && out[1] < 0.01, "{} {}", out[0], out[1]);
    // A listener that cannot be used is refused and the old one kept.
    assert!(
        m.set_listener(&Listener {
            forward: Vec3::Y,
            up: Vec3::Y,
            ..Listener::default()
        })
        .is_err()
    );
    assert!(
        m.set_listener(&Listener {
            position: Vec3::new(f32::NAN, 0.0, 0.0),
            ..Listener::default()
        })
        .is_err()
    );
    let out = render(&mut m, 480);
    assert!(out[0] > 0.99 && out[1] < 0.01, "unchanged");
}

#[test]
fn distance_attenuates_linearly_and_silence_beyond_the_range_virtualizes_the_voice() {
    let level_at = |distance: f32| {
        let mut m = mixer_with(8);
        m.add_sound(id(1), constant(1.0, 48_000));
        let v = m.play(id(1), &at(Vec3::new(0.0, 0.0, -distance))).unwrap();
        render(&mut m, 480);
        let out = render(&mut m, 480);
        (out[0] / CENTRE, m.voice_state(v).unwrap())
    };
    assert!(
        near(level_at(1.0).0, 1.0, 1e-4),
        "inside the minimum distance (2)"
    );
    assert!(near(level_at(2.0).0, 1.0, 1e-4));
    assert!(near(level_at(6.0).0, 0.5, 1e-4), "halfway between 2 and 10");
    assert!(near(level_at(9.0).0, 0.125, 1e-4));
    let (silent, state) = level_at(10.0);
    assert_eq!(silent, 0.0, "at the maximum distance");
    assert!(state.virtualized);
    let (far, state) = level_at(500.0);
    assert_eq!(far, 0.0);
    assert!(
        state.virtualized && state.position > 0.0,
        "it keeps advancing while silent: {state:?}"
    );
}

#[test]
fn update_voice_moves_and_changes_a_playing_voice_and_refuses_bad_values() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 48_000));
    let v = m.play(id(1), &at(Vec3::new(0.0, 0.0, -1.5))).unwrap();
    m.update_voice(
        v,
        &VoiceUpdate {
            position: Some(Vec3::new(1.5, 0.0, 0.0)),
            volume: Some(0.5),
            ..VoiceUpdate::default()
        },
    )
    .unwrap();
    render(&mut m, 480);
    let out = render(&mut m, 480);
    assert!(
        near(out[1], 0.5, 1e-4) && near(out[0], 0.0, 1e-4),
        "{} {}",
        out[0],
        out[1]
    );
    assert!(
        m.update_voice(
            v,
            &VoiceUpdate {
                volume: Some(-1.0),
                ..VoiceUpdate::default()
            }
        )
        .is_err()
    );
    assert!(
        m.update_voice(
            v,
            &VoiceUpdate {
                pitch: Some(9.0),
                ..VoiceUpdate::default()
            }
        )
        .is_err()
    );
    assert!(
        m.update_voice(
            v,
            &VoiceUpdate {
                position: Some(Vec3::new(f32::NAN, 0.0, 0.0)),
                ..VoiceUpdate::default()
            }
        )
        .is_err()
    );
    let after = render(&mut m, 480);
    assert!(
        near(after[1], 0.5, 1e-4),
        "a refused update changes nothing"
    );
    // A stale voice id is harmless.
    m.update_voice(
        VoiceId(9999),
        &VoiceUpdate {
            volume: Some(0.1),
            ..VoiceUpdate::default()
        },
    )
    .unwrap();
    m.stop(VoiceId(9999), 10.0);
}

#[test]
fn a_2d_voice_ignores_a_position_update() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 48_000));
    let v = m.play(id(1), &PlayParams::sfx()).unwrap();
    m.update_voice(
        v,
        &VoiceUpdate {
            position: Some(Vec3::new(50.0, 0.0, 0.0)),
            ..VoiceUpdate::default()
        },
    )
    .unwrap();
    render(&mut m, 480);
    let out = render(&mut m, 480);
    assert!(near(out[0], CENTRE, 1e-5) && near(out[1], CENTRE, 1e-5));
}

// ---- buses and ducking ------------------------------------------------------------------------

#[test]
fn bus_volumes_scale_their_voices_and_the_master_scales_everything() {
    let steady = |setup: &dyn Fn(&mut Mixer), bus: Bus| {
        let mut m = mixer_with(8);
        m.add_sound(id(1), constant(1.0, 48_000));
        m.play(id(1), &PlayParams::sfx().on_bus(bus)).unwrap();
        setup(&mut m);
        render(&mut m, 480);
        render(&mut m, 480)[0] / CENTRE
    };
    assert!(near(steady(&|_| {}, Bus::Sfx), 1.0, 1e-5));
    assert!(near(
        steady(&|m| m.set_bus_volume(Bus::Sfx, 0.5), Bus::Sfx),
        0.5,
        1e-5
    ));
    assert!(
        near(
            steady(&|m| m.set_bus_volume(Bus::Music, 0.5), Bus::Sfx),
            1.0,
            1e-5
        ),
        "another bus does not matter"
    );
    assert!(
        near(
            steady(
                &|m| {
                    m.set_bus_volume(Bus::Master, 0.5);
                    m.set_bus_volume(Bus::Sfx, 0.5)
                },
                Bus::Sfx
            ),
            0.25,
            1e-5
        ),
        "master times bus"
    );
    assert!(
        near(
            steady(&|m| m.set_bus_volume(Bus::Master, 0.5), Bus::Master),
            0.5,
            1e-5
        ),
        "a voice on the master bus is scaled once"
    );
    assert!(near(
        steady(&|m| m.set_bus_volume(Bus::Voice, 0.0), Bus::Voice),
        0.0,
        1e-6
    ));
    assert!(near(
        steady(&|m| m.set_bus_volume(Bus::Ui, 0.25), Bus::Ui),
        0.25,
        1e-5
    ));
}

#[test]
fn bus_volumes_are_clamped_and_nan_is_ignored() {
    let mut m = mixer_with(8);
    assert_eq!(Bus::ALL.map(|b| m.bus_volume(b)), [1.0; 5]);
    m.set_bus_volume(Bus::Sfx, 7.0);
    assert_eq!(m.bus_volume(Bus::Sfx), 1.0);
    m.set_bus_volume(Bus::Sfx, -3.0);
    assert_eq!(m.bus_volume(Bus::Sfx), 0.0);
    m.set_bus_volume(Bus::Sfx, 0.4);
    m.set_bus_volume(Bus::Sfx, f32::NAN);
    assert_eq!(m.bus_volume(Bus::Sfx), 0.4, "NaN changes nothing");
    m.set_bus_volume(Bus::Music, f32::INFINITY);
    assert_eq!(m.bus_volume(Bus::Music), 1.0);
}

#[test]
fn the_music_bus_ducks_under_a_voice_and_comes_back() {
    let mut m = Mixer::new(MixerConfig {
        ducking: Ducking {
            music_gain: 0.25,
            attack_ms: 100.0,
            release_ms: 500.0,
        },
        ..MixerConfig::default()
    })
    .unwrap();
    m.add_sound(id(1), constant(1.0, 48_000)); // music
    m.add_sound(id(2), constant(0.0, 480_000)); // a silent voice-over: audible to the mixer, adds nothing to the samples
    m.play(id(1), &PlayParams::sfx().on_bus(Bus::Music).looped())
        .unwrap();
    render(&mut m, 4_800);
    let before = render(&mut m, 480)[0] / CENTRE;
    assert!(near(before, 1.0, 1e-4), "{before}");
    let line = m
        .play(id(2), &PlayParams::sfx().on_bus(Bus::Voice))
        .unwrap();
    // Within several attack times (the approach is exponential) the music is at the ducked gain.
    for _ in 0..60 {
        render(&mut m, 480);
    }
    let ducked = render(&mut m, 480)[0] / CENTRE;
    assert!(near(ducked, 0.25, 0.01), "{ducked}");
    // It is going down smoothly, not in one step: halfway through the attack it is between.
    let mut m2 = Mixer::new(MixerConfig {
        ducking: Ducking {
            music_gain: 0.25,
            attack_ms: 100.0,
            release_ms: 500.0,
        },
        ..MixerConfig::default()
    })
    .unwrap();
    m2.add_sound(id(1), constant(1.0, 48_000));
    m2.add_sound(id(2), constant(0.0, 480_000));
    m2.play(id(1), &PlayParams::sfx().on_bus(Bus::Music).looped())
        .unwrap();
    render(&mut m2, 4_800);
    m2.play(id(2), &PlayParams::sfx().on_bus(Bus::Voice))
        .unwrap();
    for _ in 0..10 {
        render(&mut m2, 480); // 100 ms
    }
    let middle = render(&mut m2, 480)[0] / CENTRE;
    assert!(middle > 0.3 && middle < 0.9, "{middle}");
    // When the voice stops, the music returns.
    m.stop(line, 0.0);
    for _ in 0..300 {
        render(&mut m, 480);
    }
    let back = render(&mut m, 480)[0] / CENTRE;
    assert!(near(back, 1.0, 0.01), "{back}");
}

#[test]
fn ducking_lowers_only_the_music_bus() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 48_000)); // a sound effect
    m.add_sound(id(2), constant(0.0, 480_000)); // a voice-over
    m.play(id(1), &PlayParams::sfx().looped()).unwrap();
    m.play(id(2), &PlayParams::sfx().on_bus(Bus::Voice))
        .unwrap();
    for _ in 0..60 {
        render(&mut m, 480);
    }
    assert!(
        near(render(&mut m, 480)[0] / CENTRE, 1.0, 1e-4),
        "effects are not ducked"
    );
}

#[test]
fn a_critical_voice_over_that_is_muted_does_not_duck_the_music() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 48_000));
    m.add_sound(id(2), constant(0.0, 480_000));
    m.play(id(1), &PlayParams::sfx().on_bus(Bus::Music).looped())
        .unwrap();
    // Critical voices are always mixed, even at volume 0: they must not count as speech.
    m.play(
        id(2),
        &PlayParams::sfx()
            .on_bus(Bus::Voice)
            .with_priority(Priority::CRITICAL)
            .with_volume(0.0),
    )
    .unwrap();
    for _ in 0..60 {
        render(&mut m, 480);
    }
    assert!(near(render(&mut m, 480)[0] / CENTRE, 1.0, 1e-3));
}

#[test]
fn a_voice_that_is_silent_does_not_duck_the_music_and_ducking_of_one_turns_off() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 48_000));
    m.add_sound(id(2), constant(0.0, 480_000));
    m.play(id(1), &PlayParams::sfx().on_bus(Bus::Music).looped())
        .unwrap();
    let v = m
        .play(id(2), &PlayParams::sfx().on_bus(Bus::Voice))
        .unwrap();
    m.update_voice(
        v,
        &VoiceUpdate {
            volume: Some(0.0),
            ..VoiceUpdate::default()
        },
    )
    .unwrap();
    for _ in 0..40 {
        render(&mut m, 480);
    }
    assert!(
        near(render(&mut m, 480)[0] / CENTRE, 1.0, 1e-3),
        "a muted voice-over does not duck"
    );
    let mut off = Mixer::new(MixerConfig {
        ducking: Ducking {
            music_gain: 1.0,
            ..Ducking::default()
        },
        ..MixerConfig::default()
    })
    .unwrap();
    off.add_sound(id(1), constant(1.0, 48_000));
    off.add_sound(id(2), constant(0.0, 480_000));
    off.play(id(1), &PlayParams::sfx().on_bus(Bus::Music).looped())
        .unwrap();
    off.play(id(2), &PlayParams::sfx().on_bus(Bus::Voice))
        .unwrap();
    for _ in 0..40 {
        render(&mut off, 480);
    }
    assert!(
        near(render(&mut off, 480)[0] / CENTRE, 1.0, 1e-4),
        "a music gain of 1 is no ducking"
    );
}

// ---- voice management ---------------------------------------------------------------------------

fn play_all(m: &mut Mixer, volumes: &[(f32, Priority)]) -> Vec<VoiceId> {
    volumes
        .iter()
        .map(|(volume, priority)| {
            m.play(
                id(1),
                &PlayParams::sfx()
                    .with_volume(*volume)
                    .with_priority(*priority)
                    .looped(),
            )
            .unwrap()
        })
        .collect()
}

#[test]
fn over_the_limit_the_loudest_voices_of_the_same_priority_stay_audible() {
    let mut m = mixer_with(2);
    m.add_sound(id(1), constant(0.01, 48_000));
    let v = play_all(
        &mut m,
        &[
            (0.1, Priority::NORMAL),
            (0.3, Priority::NORMAL),
            (0.2, Priority::NORMAL),
        ],
    );
    render(&mut m, 480);
    let s = m.stats();
    assert_eq!((s.audible, s.virtualized), (2, 1));
    let virtual_flags: Vec<bool> = v
        .iter()
        .map(|id| m.voice_state(*id).unwrap().virtualized)
        .collect();
    assert_eq!(
        virtual_flags,
        [true, false, false],
        "the quietest one is virtualized"
    );
}

#[test]
fn priority_beats_loudness_and_critical_voices_are_never_virtualized() {
    let mut m = mixer_with(2);
    m.add_sound(id(1), constant(0.01, 48_000));
    let v = play_all(
        &mut m,
        &[
            (1.0, Priority::LOW),
            (1.0, Priority::NORMAL),
            (0.1, Priority::HIGH),
            (0.05, Priority::CRITICAL),
            (0.05, Priority::CRITICAL),
        ],
    );
    render(&mut m, 480);
    let flags: Vec<bool> = v
        .iter()
        .map(|id| m.voice_state(*id).unwrap().virtualized)
        .collect();
    // Limit 2: HIGH and NORMAL stay (HIGH first, then the loudest of the rest); LOW goes; the two CRITICAL ones do not count against the limit.
    assert_eq!(flags, [true, false, false, false, false]);
    assert_eq!(m.stats().audible, 4, "two criticals beyond the limit");
}

#[test]
fn ties_go_to_the_older_voice() {
    let mut m = mixer_with(1);
    m.add_sound(id(1), constant(0.01, 48_000));
    let v = play_all(
        &mut m,
        &[
            (0.5, Priority::NORMAL),
            (0.5, Priority::NORMAL),
            (0.5, Priority::NORMAL),
        ],
    );
    render(&mut m, 480);
    let flags: Vec<bool> = v
        .iter()
        .map(|id| m.voice_state(*id).unwrap().virtualized)
        .collect();
    assert_eq!(flags, [false, true, true]);
}

#[test]
fn a_virtualized_voice_keeps_its_place_and_is_in_sync_when_it_returns() {
    let mut m = mixer_with(1);
    let ramp: Vec<f32> = (0..48_000).map(|i| i as f32 / 48_000.0).collect();
    m.add_sound(id(1), SampleBuffer::new(RATE, 1, ramp).unwrap());
    let loud = m.play(id(1), &PlayParams::sfx().with_volume(1.0)).unwrap();
    let quiet = m.play(id(1), &PlayParams::sfx().with_volume(0.5)).unwrap();
    render(&mut m, 4_800);
    assert!(m.voice_state(quiet).unwrap().virtualized);
    assert!(
        near(m.voice_state(quiet).unwrap().position as f32, 4_800.0, 0.01),
        "advanced while virtual: {:?}",
        m.voice_state(quiet)
    );
    m.stop(loud, 0.0);
    render(&mut m, 480); // the loud one fades out (10 ms) and is still counted until it is gone
    assert!(m.voice_state(quiet).unwrap().virtualized);
    render(&mut m, 480); // now the quiet one is mixed
    assert!(!m.voice_state(quiet).unwrap().virtualized);
    assert!(
        near(
            m.voice_state(quiet).unwrap().position as f32,
            4_800.0 + 960.0,
            0.01
        ),
        "{:?}",
        m.voice_state(quiet)
    );
}

#[test]
fn a_voice_that_ends_while_virtual_is_removed() {
    let mut m = mixer_with(1);
    m.add_sound(id(1), constant(0.5, 48_000));
    m.add_sound(id(2), constant(0.5, 100));
    m.play(id(1), &PlayParams::sfx().with_volume(1.0)).unwrap();
    let short = m.play(id(2), &PlayParams::sfx().with_volume(0.1)).unwrap();
    render(&mut m, 480);
    assert!(
        !m.is_playing(short),
        "a virtualized one-shot still ends on time"
    );
    assert_eq!(m.voice_count(), 1);
}

#[test]
fn the_number_of_voices_alive_is_limited() {
    let mut m = mixer_with(4);
    m.add_sound(id(1), constant(0.0, 10));
    for _ in 0..ss2_audio::mixer::MAX_VOICES_ALIVE {
        m.play(id(1), &PlayParams::sfx().looped()).unwrap();
    }
    assert!(m.play(id(1), &PlayParams::sfx()).is_err());
    render(&mut m, 480);
    assert_eq!(m.stats().audible, 4, "all but four are virtualized");
    assert_eq!(
        m.stats().virtualized,
        ss2_audio::mixer::MAX_VOICES_ALIVE - 4
    );
}

// ---- stopping, suspending, errors -----------------------------------------------------------------

#[test]
fn stop_fades_the_voice_out_over_the_requested_time_and_removes_it() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 480_000));
    let v = m.play(id(1), &PlayParams::sfx()).unwrap();
    render(&mut m, 480);
    m.stop(v, 100.0); // 4800 frames
    assert!(m.voice_state(v).unwrap().stopping);
    let out = render(&mut m, 2_400);
    assert!(
        near(out[0], CENTRE, 0.01),
        "just started fading: {}",
        out[0]
    );
    assert!(
        near(out[2 * 1_200], CENTRE * 0.75, 0.01),
        "a quarter of the way: {}",
        out[2 * 1_200]
    );
    assert!(m.is_playing(v));
    let rest = render(&mut m, 2_400);
    assert!(rest[2 * 2_399].abs() < 0.01);
    assert!(!m.is_playing(v), "gone after the fade");
    assert!(render(&mut m, 480).iter().all(|s| *s == 0.0));
    m.stop(v, 100.0); // a finished voice: nothing happens
}

#[test]
fn a_stop_without_a_fade_still_takes_the_declick_time() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 480_000));
    let v = m.play(id(1), &PlayParams::sfx()).unwrap();
    render(&mut m, 480);
    m.stop(v, 0.0);
    let out = render(&mut m, 480); // 10 ms
    assert!(
        near(out[2 * 240], CENTRE * 0.5, 0.02),
        "half way through the declick it is half: {}",
        out[2 * 240]
    );
    assert!(
        out[0] > 0.5 && out[2 * 479].abs() < 0.01,
        "a ramp, not a cut: {} .. {}",
        out[0],
        out[2 * 479]
    );
    assert!(!m.is_playing(v));
    let nan = m.play(id(1), &PlayParams::sfx()).unwrap();
    m.stop(nan, f32::NAN);
    assert!(
        m.voice_state(nan).unwrap().stopping,
        "a bad fade time falls back to the declick"
    );
}

#[test]
fn suspending_fades_to_silence_without_dropping_voices_and_resuming_brings_the_sound_back() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 480_000));
    let v = m.play(id(1), &PlayParams::sfx()).unwrap();
    render(&mut m, 480);
    m.set_suspended(true);
    let fade = render(&mut m, 480); // the declick time
    assert!(
        fade[0] > 0.3 && fade[2 * 479].abs() < 0.01,
        "{} .. {}",
        fade[0],
        fade[2 * 479]
    );
    let before = m.voice_state(v).unwrap().position;
    let silent = render(&mut m, 4_800);
    assert!(silent.iter().all(|s| *s == 0.0), "silent while suspended");
    assert!(
        m.is_playing(v) && m.voice_state(v).unwrap().position > before,
        "the voice goes on"
    );
    m.set_suspended(false);
    let resume = render(&mut m, 480);
    assert!(
        near(resume[2 * 239], CENTRE * 0.5, 0.02),
        "half way back after half the declick time: {}",
        resume[2 * 239]
    );
    assert!(
        near(render(&mut m, 480)[0], CENTRE, 1e-4),
        "back at full level"
    );
}

#[test]
fn the_fade_back_in_takes_the_declick_time_whatever_the_block_size() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 480_000));
    m.play(id(1), &PlayParams::sfx().looped()).unwrap();
    render(&mut m, 480);
    m.set_suspended(true);
    render(&mut m, 4_800);
    m.set_suspended(false);
    // 240 frames of a 480-frame declick: half way back, in small blocks.
    let first = render(&mut m, 240);
    assert!(
        near(first[2 * 239], CENTRE * 0.5, 0.02),
        "{}",
        first[2 * 239]
    );
    let second = render(&mut m, 240);
    assert!(near(second[2 * 239], CENTRE, 0.02), "{}", second[2 * 239]);
}

#[test]
fn bad_requests_are_refused() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 100));
    assert!(
        m.play(id(2), &PlayParams::sfx()).is_err(),
        "a sound that was never added"
    );
    assert!(m.play(id(1), &PlayParams::sfx().with_volume(-1.0)).is_err());
    assert!(m.play(id(1), &PlayParams::sfx().with_pitch(0.0)).is_err());
    assert!(
        m.play(id(1), &PlayParams::sfx().with_volume(f32::NAN))
            .is_err()
    );
    assert_eq!(m.voice_count(), 0, "a refused play leaves nothing behind");
    assert!(SampleBuffer::new(0, 1, vec![0.0]).is_err());
    assert!(SampleBuffer::new(48_000, 3, vec![0.0; 3]).is_err());
    assert!(
        SampleBuffer::new(48_000, 2, vec![0.0; 3]).is_err(),
        "not whole frames"
    );
    assert_eq!(
        SampleBuffer::new(48_000, 2, vec![0.0; 8]).unwrap().frames(),
        4
    );
    for bad in [
        MixerConfig {
            sample_rate: 0,
            ..MixerConfig::default()
        },
        MixerConfig {
            max_voices: 0,
            ..MixerConfig::default()
        },
        MixerConfig {
            declick_ms: -1.0,
            ..MixerConfig::default()
        },
        MixerConfig {
            declick_ms: f32::NAN,
            ..MixerConfig::default()
        },
        MixerConfig {
            ducking: Ducking {
                music_gain: 1.5,
                ..Ducking::default()
            },
            ..MixerConfig::default()
        },
        MixerConfig {
            ducking: Ducking {
                attack_ms: -5.0,
                ..Ducking::default()
            },
            ..MixerConfig::default()
        },
    ] {
        assert!(Mixer::new(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn an_empty_or_odd_sized_output_buffer_is_handled() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 100));
    m.play(id(1), &PlayParams::sfx().looped()).unwrap();
    let mut none: [f32; 0] = [];
    m.render(&mut none);
    let mut odd = [9.0f32; 5];
    m.render(&mut odd);
    assert_eq!(odd[4], 0.0, "the odd last sample is left silent");
}

// ---- the sum, determinism and the offline renderer -------------------------------------------------

#[test]
fn many_loud_voices_are_clamped_and_counted() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 48_000));
    for _ in 0..4 {
        m.play(id(1), &PlayParams::sfx().looped()).unwrap();
    }
    render(&mut m, 480);
    let out = render(&mut m, 480);
    assert!(
        out.iter().all(|s| (-1.0..=1.0).contains(s)),
        "never outside the range"
    );
    assert!(out.iter().all(|s| *s == 1.0), "4 x 0.707 is clamped to 1");
    assert!(m.stats().clipped_samples >= 960, "{:?}", m.stats());
}

#[test]
fn the_sum_is_clamped_as_soon_as_it_leaves_the_range() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(1.0, 48_000));
    for _ in 0..2 {
        m.play(id(1), &PlayParams::sfx().looped()).unwrap();
    }
    render(&mut m, 480);
    let out = render(&mut m, 480);
    assert!(
        out.iter().all(|s| *s == 1.0),
        "2 x 0.707 = 1.414 is clamped: {}",
        out[0]
    );
    assert!(m.stats().clipped_samples > 0);
    // A sum inside the range is left alone and not counted.
    let mut quiet = mixer_with(8);
    quiet.add_sound(id(1), constant(1.0, 48_000));
    quiet
        .play(id(1), &PlayParams::sfx().looped().with_volume(1.0))
        .unwrap();
    render(&mut quiet, 480);
    assert_eq!(quiet.stats().clipped_samples, 0);
}

#[test]
fn the_same_calls_give_the_same_samples() {
    let scene = || {
        let mut m = mixer_with(3);
        let ramp: Vec<f32> = (0..20_000)
            .map(|i| ((i * 7919) % 2000) as f32 / 1000.0 - 1.0)
            .collect();
        m.add_sound(id(1), SampleBuffer::new(RATE, 1, ramp).unwrap());
        for i in 0..6 {
            m.play(
                id(1),
                &at(Vec3::new(i as f32 - 3.0, 0.0, -(i as f32)))
                    .with_pitch(0.7 + i as f32 * 0.2)
                    .looped()
                    .with_volume(0.3),
            )
            .unwrap();
        }
        render_offline(&mut m, 30_000, 1024)
    };
    let (a, b) = (scene(), scene());
    assert!(a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()));
    assert!(a.iter().any(|s| s.abs() > 0.01), "and it is not silence");
}

#[test]
fn the_offline_renderer_fills_the_requested_length_in_any_block_size_and_writes_a_wav() {
    let mut m = mixer_with(8);
    m.add_sound(id(1), constant(0.25, 48_000));
    m.play(id(1), &PlayParams::sfx().looped()).unwrap();
    for block in [1, 7, 480, 100_000] {
        let mut copy = mixer_with(8);
        copy.add_sound(id(1), constant(0.25, 48_000));
        copy.play(id(1), &PlayParams::sfx().looped()).unwrap();
        assert_eq!(
            render_offline(&mut copy, 1_001, block).len(),
            2_002,
            "block {block}"
        );
    }
    assert!(render_offline(&mut m, 0, 480).is_empty());
    let out = render_offline(&mut m, 4_800, 480);
    let bytes = wav::encode_pcm16(RATE, 2, &out).unwrap();
    let decoded = wav::decode(&bytes).unwrap();
    assert_eq!(
        (decoded.rate, decoded.channels, decoded.samples.len()),
        (RATE, 2, 9_600)
    );
    assert!(
        AudioTolerance {
            max_abs: 1.0 / 32767.0 + 1e-6,
            rms: 1e-4
        }
        .accepts(&compare(&out, &decoded.samples).unwrap())
    );
    let float = wav::decode(&wav::encode_f32(RATE, 2, &out).unwrap()).unwrap();
    assert_eq!(compare(&out, &float.samples).unwrap().max_abs, 0.0);
}

#[test]
fn a_golden_scene_matches_what_the_arithmetic_predicts() {
    // Two sines in two places for one second: the mix is the sum of each scaled by its own gains. This is the golden-audio test of the
    // renderer without a stored file: the expectation is computed independently from the source samples.
    let sine = |freq: f32, n: usize| {
        0.4 * (2.0 * std::f32::consts::PI * freq * n as f32 / RATE as f32).sin()
    };
    let a: Vec<f32> = (0..RATE as usize).map(|n| sine(440.0, n)).collect();
    let b: Vec<f32> = (0..RATE as usize).map(|n| sine(660.0, n)).collect();
    let mut m = mixer_with(8);
    m.add_sound(id(1), SampleBuffer::new(RATE, 1, a.clone()).unwrap());
    m.add_sound(id(2), SampleBuffer::new(RATE, 1, b.clone()).unwrap());
    m.play(id(1), &at(Vec3::new(1.5, 0.0, 0.0))).unwrap(); // hard right, full level
    m.play(id(2), &PlayParams::sfx().with_volume(0.5)).unwrap(); // centred at half level
    let out = render_offline(&mut m, 24_000, 512);
    let skip = 512; // the fade-in block
    let mut expected = Vec::new();
    for n in skip..24_000 {
        expected.push(b[n] * 0.5 * CENTRE); // left: only the centred voice
        expected.push(a[n] + b[n] * 0.5 * CENTRE); // right: the right-hand voice and the centred one
    }
    let diff = compare(&out[2 * skip..], &expected).unwrap();
    assert!(AudioTolerance::default().accepts(&diff), "{diff:?}");
}
