use std::env;
use std::fs::File;
use std::path::Path;

use ansi_term::Style;
use rustfft::{Fft, FftDirection, FftPlanner, num_complex::Complex};
use rustfft::algorithm::Radix4;

static ARG_FILE: &'static str = "--file";
static ARG_HELP: &'static str = "--help";

static N_ONSET: usize = 2048;
static M_ONSET_SIGNAL_ENVELOPE: usize = 10;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 { // no custom params given
        print_help();
    } else if args.len() > 1 { // custom params given
        let mut i = 1;
        while i < args.len() {
            if args[i].eq(&ARG_FILE) {
                i += 1;
                print!("{}", args[i]);
                process_file(Path::new(&args[i]));
            } else if args[i].eq(&ARG_HELP) {
                print_help();
            }
            i += 1;
        }
    }
}


fn print_help() {
    #[cfg(target_os = "windows")]
    {
        ansi_term::enable_ansi_support(); // enable the super fancy output on windows!!
    }

    println!("{}{}{}", Style::new().bold().paint("404 - music "), Style::new().bold().strikethrough().paint("not"), Style::new().bold().paint(" found"));
    println!("");
    println!("Please use the following parameters when using this tool:");
    println!("  {} <path>      Process the passed file", ARG_FILE);
    println!("  --folder <path>    Process all .wav files in the passed folder");
    println!("  --out <path>       Where to save the reuslt JSON file");
    println!("  --train <path>     Compute the f-measure on the .gt files of the passed folder");
    println!("  --verbose          Display a lot of output status messages");
    println!("  {}             Displays this help message", ARG_HELP);
    println!("  ... more to be added");
}

fn process_file(file_path: &Path) {
    let input_file = File::open(&file_path).unwrap();
    let (header, samples) = wav_io::read_from_file(input_file).unwrap();
    println!("header={:?}", header);
    println!("Sample rate: {} Hz", header.sample_rate);

    // print the first 32 values of the sample for testing
    for (i, v) in samples.iter().enumerate() {
        println!("{}: {}v", i, v);
        if i > 32 {
            break;
        }
    }
    println!("Sample Länge: {}", samples.len());

    let mut cur_pos: usize = 0;
    while cur_pos + N_ONSET < samples.len() {
        /* FFT realisation based on https://github.com/ejmahler/RustFFT/blob/master/UpgradeGuide4to5.md */
        let mut fft_buffer: Vec<Complex<f32>> = samples[cur_pos..cur_pos + N_ONSET].iter().map(|&value| Complex::new(value, 0f32)).into_iter().collect::<Vec<_>>();
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(N_ONSET);
        fft.process(&mut fft_buffer);
        cur_pos += 1024;
    }
    let mut fft_buffer: Vec<Complex<f32>> = samples[cur_pos..samples.len()].iter().map(|&value| Complex::new(value, 0f32)).into_iter().collect::<Vec<_>>();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(samples.len() - cur_pos);
    fft.process(&mut fft_buffer); // fft_buffer contains the transformed part of the signal...

    // everything run through the fourier transform, but the results are not used yet.

    print!("Hi");
}