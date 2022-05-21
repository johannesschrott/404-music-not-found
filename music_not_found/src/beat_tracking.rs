use std::borrow::Borrow;
use arima::acf::acf;

const BEAT_ACCURACY: f64 = 70e-3;

pub fn get_beats(onset_times: &Vec<f32>) -> Vec<f64> {
    let mut times: Vec<f64> = Vec::new();
    let mut a_corr: Vec<f64> = Vec::new();


    for &onset_time in onset_times.iter() {
        times.push(onset_time as f64 - (onset_time as f64 % (BEAT_ACCURACY / 4.)));
    }

    let times_slice: &[f64] = times.as_slice();
    match acf(times_slice ,Some(times.len()), false)  {
        Ok(auto_corr) => {a_corr = auto_corr} ,
        Err(e) => println!("{:?}", e),
    };

    return a_corr;
}