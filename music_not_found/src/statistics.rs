use dsp::window;
use rustfft::{num_complex::Complex, FftPlanner};
use std::iter::repeat;

pub fn normalize(data: &[f32]) -> Vec<f32> {
    let max = data.iter().cloned().fold(0. / 0., f32::max);
    let mean = data.iter().sum::<f32>() / data.len() as f32;

    data.into_iter().map(|x| (x - mean) / max).collect()
}

pub fn convolve1D<F>(data: &Vec<f32>, kernel_size: usize, kernel_function: F) -> Vec<f32>
where
    F: Fn(&[f32]) -> f32,
{
    let half = kernel_size / 2;

    let mut output = Vec::new();

    for i in 0..data.len() {
        if i >= half && i < data.len() - half {
            output.push(kernel_function(&data[i - half..=i + half]));
        } else if i >= half {
            let mut first = data[i..].to_owned();
            let mut second = zeroes(kernel_size + i - data.len());
            first.append(&mut second);
            output.push(kernel_function(&first[..]));
        } else if i < data.len() - half {
            let mut second = data[0..=i + half].to_owned();
            let mut first = zeroes(kernel_size - i - half);
            first.append(&mut second);
            output.push(kernel_function(&first[..]));
        } else {
            panic!("Kernel size must be smaller than data size!");
        }
    }

    output
}

pub fn zeroes(n: usize) -> Vec<f32> {
    repeat(0.).take(n).collect()
}

pub fn vec_add(vecs: &[&[f32]]) -> Vec<f32> {
    let mut output: Vec<f32> = Vec::new();

    for _ in 0..vecs[0].len() {
        output.push(0.);
    }

    for v in vecs.into_iter() {
        v.iter().enumerate().for_each(|(i, x)| output[i] += x)
    }
    output
}

pub fn vec_mult(vecs: &[&[f32]]) -> Vec<f32> {
    let mut output: Vec<f32> = Vec::new();

    for _ in 0..vecs[0].len() {
        output.push(1.);
    }

    for v in vecs.into_iter() {
        v.iter().enumerate().for_each(|(i, x)| output[i] *= x)
    }
    output
}

pub fn stft(data: &[f32], window_size: usize, hop_size: usize) -> Vec<Vec<Complex<f32>>> {
    let mut planner = FftPlanner::new();
    let hamming = window::hamming(window_size);

    let fft = planner.plan_fft_forward(window_size);
    //   let samples: Vec<Complex<f32>> = track
    //       .samples
    //      .iter()
    //     .map(|&value| Complex::new(value, 0f32))
    //      .collect();

    let mut stft = Vec::new();

    let mut cur_pos: usize = 0;
    while cur_pos + window_size < data.len() {
        let mut fft_buffer_real = vec![0f32; window_size];
        let fft_in = &data[cur_pos..cur_pos + window_size];

        hamming.apply(fft_in, &mut fft_buffer_real);

        let mut fft_buffer_comp: Vec<Complex<f32>> = fft_buffer_real
            .iter()
            .map(|&value| Complex::new(value, 0f32))
            .collect();
        fft.process(&mut fft_buffer_comp);
        cur_pos += hop_size; // TODO: evtl. nicht um /2 sonden um ganzen N_ONSET verschieben
        stft.push(fft_buffer_comp);
    }

    let mut fft_in: Vec<f32> = data[cur_pos..data.len() - 1].to_owned();
    fft_in.extend(repeat(0f32).take(window_size - fft_in.len()));
    let mut fft_buffer_real = vec![0f32; window_size];
    hamming.apply(&fft_in, &mut fft_buffer_real);

    let mut fft_buffer_comp: Vec<Complex<f32>> = fft_buffer_real
        .iter()
        .map(|&value| Complex::new(value, 0f32))
        .collect();
    fft.process(&mut fft_buffer_comp);
    stft.push(fft_buffer_comp);
    stft
}