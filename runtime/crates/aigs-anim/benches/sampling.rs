//! Animation sampling benchmarks. Baselines in `docs/testing.md`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use aigs_anim::{sample, Easing, Keyframe};

fn track(keyframes: usize) -> Vec<Keyframe> {
    (0..keyframes)
        .map(|i| Keyframe {
            frame: (i * 10) as u32,
            value: i as f32,
            easing: Easing::EaseInOut,
        })
        .collect()
}

fn bench_sampling(c: &mut Criterion) {
    let short = track(8);
    let long = track(256);
    c.bench_function("sample_8kf", |b| {
        b.iter(|| black_box(sample(&short, black_box(37.5))))
    });
    c.bench_function("sample_256kf", |b| {
        b.iter(|| black_box(sample(&long, black_box(1_275.0))))
    });
}

criterion_group!(benches, bench_sampling);
criterion_main!(benches);
