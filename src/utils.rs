use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

fn get_hann_window(size: usize) -> Vec<f32> {
    (0..size)
        .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / size as f32).cos())
        .collect()
}

fn get_buffer_for_fft(samples: &Vec<f32>, hann_window: Vec<f32>) -> Vec<Complex<f32>> {
    samples
        .iter()
        .zip(hann_window.iter())
        .map(|(&sample, &window_mult)| Complex {
            re: sample * window_mult,
            im: 0.0,
        })
        .collect()
}

pub fn get_mag_spectrum(samples: &Vec<f32>, sample_rate: u32) -> Vec<(f64, f64)> {
    let size = samples.len();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(size);

    let hann_window = get_hann_window(size);
    let mut buffer = get_buffer_for_fft(&samples, hann_window);

    fft.process(&mut buffer);

    buffer
        .split_at(size / 2)
        .0
        .iter()
        .enumerate()
        .map(|(i, c)| {
            (
                (i as f64 * sample_rate as f64 / size as f64),
                c.norm() as f64,
            )
        })
        .collect()
}
