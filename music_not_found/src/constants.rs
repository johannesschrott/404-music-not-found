
/// The number of Mel Bands used at LFSF
pub const MEL_BANDS: usize = 128;


/* For validation of results */
/// Accuracy in seconds of the estimated onsets
pub const ONSET_ACCURACY: f64 = 50e-3;
/// Accuracy in seconds of the estimated beats
pub const BEAT_ACCURACY: f64 = 70e-3;
/// Deviation of which the estimated tempo may be different (+ and -)
pub const TEMPO_DEVIATION: f64 = 0.08;


/* For tempo estimation */
pub const SLOWEST_BPM: f64 = 60.;
pub const HIGHEST_BPM: f64 = 200.;