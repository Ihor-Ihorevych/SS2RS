//! Audio-contract hot paths: starting and updating many voices under a voice limit.

use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ss2_audio_api::{Audio, NullAudio, PlayParams, Priority, VoiceUpdate};
use ss2_core::math::Vec3;

fn voices(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio");

    // A firefight: 256 one-shots against a 32-voice budget, so culling runs constantly.
    group.bench_function("play_256_with_32_voice_limit", |b| {
        b.iter_batched(
            || NullAudio::with_voice_limit(32),
            |mut audio| {
                let sound = audio.resources().sound("sounds/weapons/shotgun.wav");
                for i in 0..256_u32 {
                    let priority = Priority(u8::try_from(i % 4 * 64).unwrap_or(255));
                    let params = PlayParams::sfx()
                        .at(Vec3::new(i as f32, 0.0, -(i as f32)))
                        .with_priority(priority);
                    black_box(audio.play(sound, &params).unwrap());
                }
                audio
            },
            BatchSize::SmallInput,
        );
    });

    // Moving 32 emitters every frame.
    group.bench_function("update_32_voices", |b| {
        let mut audio = NullAudio::new();
        let sound = audio.resources().sound("sounds/ambient/wind.wav");
        let ids: Vec<_> = (0..32)
            .map(|_| {
                audio
                    .play(sound, &PlayParams::sfx().at(Vec3::ZERO).looped())
                    .unwrap()
            })
            .collect();
        let mut x = 0.0_f32;
        b.iter(|| {
            x += 0.1;
            let update = VoiceUpdate {
                position: Some(Vec3::new(x, 0.0, 0.0)),
                ..Default::default()
            };
            for id in &ids {
                audio.update_voice(*id, &update).unwrap();
            }
        });
        audio.stop(ids[0], Duration::ZERO);
    });
    group.finish();
}

criterion_group!(benches, voices);
criterion_main!(benches);
