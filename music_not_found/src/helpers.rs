use std::iter::repeat;

use dsp::window;
use rustfft::{FftPlanner, num_complex::Complex};

/// Creates vectors of given length only containing zeroes
pub fn zeroes(n: usize) -> Vec<f32> {
    repeat(0.).take(n).collect()
}

/// Computes the stft of the given signal, using the given window and hop-size
pub fn stft(signal: &[f32], window_size: usize, hop_size: usize) -> WinVec<Vec<Complex<f32>>> {
    let mut planner = FftPlanner::new();
    let hamming = window::hamming(window_size);

    let fft = planner.plan_fft_forward(window_size);

    let mut stft = Vec::new();  // Vector containing the computed FFTs

    let mut cur_pos: usize = 0;
    // Compute FFTs of the window size; at the end of iteration shift the window (hop-size!)
    while cur_pos + window_size < signal.len() {
        let mut fft_buffer_real = vec![0f32; window_size];
        let fft_in = &signal[cur_pos..cur_pos + window_size];

        hamming.apply(fft_in, &mut fft_buffer_real);

        let mut fft_buffer_comp: Vec<Complex<f32>> = fft_buffer_real
            .iter()
            .map(|&value| Complex::new(value, 0f32))
            .collect();
        fft.process(&mut fft_buffer_comp);
        cur_pos += hop_size;
        stft.push(fft_buffer_comp);
    }

    // The last fft window may be too short --> zero padding is added before fft computation
    let mut fft_in: Vec<f32> = signal[cur_pos..signal.len() - 1].to_owned();
    fft_in.extend(repeat(0f32).take(window_size - fft_in.len()));
    let mut fft_buffer_real = vec![0f32; window_size];
    hamming.apply(&fft_in, &mut fft_buffer_real);

    let mut fft_buffer_comp: Vec<Complex<f32>> = fft_buffer_real
        .iter()
        .map(|&value| Complex::new(value, 0f32))
        .collect();
    fft.process(&mut fft_buffer_comp);
    stft.push(fft_buffer_comp);

    WinVec {
        data: stft,
        window_size,
        hop_size,
    }
}

/// WinVec<A> is a Wrapper over Vec<A> which keeps track of the used windows size and the hop size
/// That way, we can try easily with different window sizes at the same time
#[derive(Clone, Debug)]
pub struct WinVec<A> {
    pub window_size: usize,
    pub hop_size: usize,
    pub data: Vec<A>,
}

impl<A> WinVec<A> {
    // Map the content of WinVec without changing hop_size or window_size
    pub fn map<F, B>(&self, f: F) -> WinVec<B>
        where
            F: Fn(&Vec<A>) -> Vec<B>,
    {
        WinVec {
            data: f(&self.data),
            window_size: self.window_size,
            hop_size: self.hop_size,
        }
    }

    // Set the content of WinVec without changing hop_size or window_size
    pub fn set_data<B>(&self, data: Vec<B>) -> WinVec<B> {
        WinVec {
            data: data,
            window_size: self.window_size,
            hop_size: self.hop_size,
        }
    }
}