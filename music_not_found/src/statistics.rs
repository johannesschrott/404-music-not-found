use std::iter::repeat;

pub fn normalize(data: &[f32]) -> Vec<f32> {
    let max = data.iter().cloned().fold(0. / 0., f32::max);
    let min = data.iter().cloned().fold(0. / 0., f32::min);
    let diff = max - min;

    data.into_iter().map(|x| (x - min) / diff).collect()
}

pub fn convolve1D<F>(data: &Vec<f32>, kernel_size: usize, kernel_function: F) -> Vec<f32>
where
    F: Fn(&[f32]) -> f32,
{
    let half = kernel_size / 2;

    let mut output = Vec::new();

    for i in 0..data.len() {
        if i >= half && i < data.len() - half {
            output.push(kernel_function(&data[i - half..=i+half]));
        } else if i >= half {
            let mut first = data[i..].to_owned();
            let mut second = zeroes(kernel_size + i - data.len());
            first.append(&mut second);
            output.push(kernel_function(&first[..]));
        } else if i < data.len() - half {
            let mut second = data[0..=i+half].to_owned();
            let mut first = zeroes(kernel_size - i - half);
            first.append(&mut second);
            output.push(kernel_function(&first[..]));   
        } else {
            panic!("Kernel size must be smaller than data size!");
        }
    }

    output
}

pub fn zeroes(n: usize) -> Vec<f32>{
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
