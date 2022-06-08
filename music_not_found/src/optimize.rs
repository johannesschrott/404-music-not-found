// TODO: wird aus der Klasse überhaupt wo etwas verwendet??

use serde::*;
use simple_optimization::grid_search;

use crate::{peak_picking::PeakPicker, track::Track};

#[derive(Serialize, Deserialize)]
pub struct Parameters<A> {
    pub peak_picker: PeakPicker,
    pub method: A,
}
