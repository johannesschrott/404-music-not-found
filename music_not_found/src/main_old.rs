use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

mod track;

use ansi_term::Style;
use rustfft::{Fft, FftDirection, FftPlanner, num_complex::Complex};
use rustfft::algorithm::Radix4;

static ARG_FILE: &'static str = "--file";
static ARG_HELP: &'static str = "--help";
static ARG_VALIDATE: &'static str = "--validate";

static N_ONSET: usize = 2048;
static M_ONSET_SIGNAL_ENVELOPE: usize = 10;

/// Accuracy in seconds of the estimated onsets
static ONSET_ACCURACY: f64 = 50e-3;
/// Accuracy in seconds of the estimated beats
static BEAT_ACCURACY: f64 = 70e-3;
/// Deviation of which the estimated tempo may be different (+ and -)
static TEMPO_DEVIATION: f64 = 0.08;

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
//    println!("  {} <path>  Compute the f-measure on the .gt files of the passed folder", ARG_VALIDATE);
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
        cur_pos += (N_ONSET / 2); // TODO: evtl. nicht um /2 sonden um ganzen N_ONSET verschieben
    }
    let mut fft_buffer: Vec<Complex<f32>> = samples[cur_pos..samples.len()].iter().map(|&value| Complex::new(value, 0f32)).into_iter().collect::<Vec<_>>();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(samples.len() - cur_pos);
    fft.process(&mut fft_buffer); // fft_buffer contains the transformed part of the signal...

    // everything run through the fourier transform, but the results are not used yet.

    // Check if there is an *.onsets.gt file for the wav
    let file_string_onsets_gt = [file_path.to_str().unwrap().strip_suffix(".wav").unwrap(), ".onsets.gt"].join("");

    let mut found_onsets: Vec<f64> = Vec::new();


    // onset detection should happen here (found onsets seconds have to be pushed to the vector)

    found_onsets.push(0.2f64);


    if Path::new(&file_string_onsets_gt).exists() { // if a onsets.gt file in the same folder exists, do a validation!
        println!("Validation of Found onsets");
        let gt_file = File::open(Path::new(&file_string_onsets_gt)).unwrap();
        let reader = BufReader::new(gt_file);

        /// Vector containing the true onset times (in seconds!)
        let gt_onsets: Vec<f64> = reader.lines().map(|line| line.expect("Error on parsing line")).map(|line| line.parse::<f64>().unwrap()).collect();


        f_measure_onsets(&found_onsets, &gt_onsets);
    }
}

fn f_measure_onsets(found_onsets: &Vec<f64>, gt_onsets: &Vec<f64>) {
    /// current index in vector of found onsets
    let mut i_found: usize = 0;
    /// current index in vector of gt onsets
    let mut i_gt: usize = 0;

    let mut t_p: usize = 0;
    let t_n: usize = 0; // There are no true negatives!
    let mut f_p: usize = 0;
    let mut f_n: usize = 0;

    if found_onsets.len() == 0 && gt_onsets.len() != 0 {
        println!("No onsets found :( Something may have gone wrong");
        return;
    }
    while i_found < found_onsets.len() && i_gt < gt_onsets.len() {
        if gt_onsets[i_gt] - ONSET_ACCURACY <= found_onsets[i_found] && found_onsets[i_found] <= gt_onsets[i_gt] + ONSET_ACCURACY {
            // the found onset is within the accuracy border
            t_p += 1;
            i_found += 1;
            i_gt += 1;
        } else if found_onsets[i_found] < gt_onsets[i_gt] - ONSET_ACCURACY {
            f_p += 1;
            i_found += 1;
        } else if gt_onsets[i_gt] + ONSET_ACCURACY < found_onsets[i_found] {
            f_n += 1;
            i_gt += 1;
        }
    }
    if i_gt < gt_onsets.len() {
        f_n += gt_onsets.len() - i_gt;
    }
    let precision: f64 = t_p as f64 / (t_p as f64 + f_p as f64);
    let recall: f64 = t_p as f64 / (t_p as f64 + f_n as f64);
    let f_measure: f64 = 2f64 * (precision * recall) / (precision + recall);

    println!("Precession: {}", precision);
    println!("Recall:     {}", recall);
    println!("F-Measure:  {}", f_measure);
}