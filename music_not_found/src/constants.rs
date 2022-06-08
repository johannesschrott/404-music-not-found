/// Defines how many tracks are processed in parallel
pub const NO_THREADS: usize = 12;

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
/// Lower boundary of possible tempo
pub const SLOWEST_BPM: f64 = 60.;
/// Upper boundary of possible tempo
pub const HIGHEST_BPM: f64 = 200.;


/// Parameter that describes how onset times of different algorithms are combined.
/// 1 means an onset time needs to be found by all onset algorithms,
/// 0.5 means half of the used onset algorithms need to have an onset found in order to count it
pub const ENSEMBLE_NEEDED_SCORE: f64 = 1.;
