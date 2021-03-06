use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use crate::constants::*;

/// Structure containing F-Measure results of one track
pub struct FMeasure {
    pub precision: f64,
    pub recall: f64,
    pub f_measure: f64,
}

/// Read onsets.gt files and compute F-Measure for them
pub fn f_measure_onsets(found_onsets: &Vec<f64>, file_path: &Path) -> Option<FMeasure> {
    let file_string_onsets_gt = [
        file_path.to_str().unwrap().strip_suffix(".wav").unwrap(),
        ".onsets.gt",
    ]
        .join("");

    if !Path::new(&file_string_onsets_gt).exists() {
        // if a onsets.gt file in the same folder exists, do a validation!
        return None;
    }

    let gt_file = File::open(Path::new(&file_string_onsets_gt)).unwrap();
    let reader = BufReader::new(gt_file);

    // Vector containing the true onset times (in seconds!)
    let gt_onsets: Vec<f64> = reader
        .lines()
        .map(|line| line.expect("Error on parsing line"))
        .map(|line| line.parse::<f64>().unwrap())
        .collect();

    // current index in vector of found onsets
    let mut i_found: usize = 0;
    // current index in vector of gt onsets
    let mut i_gt: usize = 0;

    let mut t_p: usize = 0;
    // let t_n: usize = 0; // There are no true negatives!
    let mut f_p: usize = 0;
    let mut f_n: usize = 0;

    if found_onsets.len() == 0 && gt_onsets.len() != 0 {
        println!("No onsets found :( Something may have gone wrong");
        return None;
    }
    while i_found < found_onsets.len() && i_gt < gt_onsets.len() {
        if gt_onsets[i_gt] - ONSET_ACCURACY <= found_onsets[i_found]
            && found_onsets[i_found] <= gt_onsets[i_gt] + ONSET_ACCURACY
        {
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

    return Some(FMeasure {
        precision,
        recall,
        f_measure: f_measure,
    });
}

/// Read beats.gt files and compute F-Measure for them
pub fn f_measure_beats(found_beats: &Vec<f64>, file_path: &Path) -> Option<FMeasure> {
    let file_string_beats_gt = [
        file_path.to_str().unwrap().strip_suffix(".wav").unwrap(),
        ".beats.gt",
    ]
        .join("");

    if !Path::new(&file_string_beats_gt).exists() {
        // if a beats.gt file in the same folder exists, do a validation!
        return None;
    }

    let gt_file = File::open(Path::new(&file_string_beats_gt)).unwrap();
    let reader = BufReader::new(gt_file);

    // Vector containing the true onset times (in seconds!)
    let gt_beats: Vec<f64> = reader
        .lines()
        .map(|line| line.expect("Error on parsing line"))
        .map(|line| {
//            println!("{}", line.split_whitespace().collect::<Vec<&str>>()[0]);
            return line.split_whitespace().collect::<Vec<&str>>()[0].parse::<f64>().unwrap();
        })
        .collect();

    // current index in vector of found onsets
    let mut i_found: usize = 0;
    // current index in vector of gt onsets
    let mut i_gt: usize = 0;

    let mut t_p: usize = 0;
    // let t_n: usize = 0; // There are no true negatives!
    let mut f_p: usize = 0;
    let mut f_n: usize = 0;

    if found_beats.len() == 0 && gt_beats.len() != 0 {
        println!("No beats found :( Something may have gone wrong");
        return None;
    }
    while i_found < found_beats.len() && i_gt < gt_beats.len() {
        if gt_beats[i_gt] - BEAT_ACCURACY <= found_beats[i_found]
            && found_beats[i_found] <= gt_beats[i_gt] + BEAT_ACCURACY
        {
            // the found onset is within the accuracy border
            t_p += 1;
            i_found += 1;
            i_gt += 1;
        } else if found_beats[i_found] < gt_beats[i_gt] - BEAT_ACCURACY {
            f_p += 1;
            i_found += 1;
        } else if gt_beats[i_gt] + BEAT_ACCURACY < found_beats[i_found] {
            f_n += 1;
            i_gt += 1;
        }
    }
    if i_gt < gt_beats.len() {
        f_n += gt_beats.len() - i_gt;
    }
    let precision: f64 = t_p as f64 / (t_p as f64 + f_p as f64);
    let recall: f64 = t_p as f64 / (t_p as f64 + f_n as f64);
    let f_measure: f64 = 2f64 * (precision * recall) / (precision + recall);

    return Some(FMeasure {
        precision,
        recall,
        f_measure,
    });
}
