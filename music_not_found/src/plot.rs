use plotters::prelude::*;
pub(crate) fn plot(data: &Vec<f32>, filename: &str) {
    let max = data.iter().cloned().fold(0. / 0., f32::max);
    let min = data.iter().cloned().fold(0. / 0., f32::min);

    let root = BitMapBackend::new(filename, (640, 480)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f32..data.len() as f32, min..max)
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    chart
        .draw_series(LineSeries::new(
            (0..data.len()).map(|x| (x as f32, data[x])),
            // (0..data.len()).into_iter().map(|x| x as f32).zip(data.iter()),
            &RED,
        ))
        .unwrap();
}
