use super::*;
use plotlib::page::Page;
use plotlib::repr::Plot;
use plotlib::style::{LineStyle, PointMarker, PointStyle};
use plotlib::view::ContinuousView;

pub struct Grapher<'a> {
    out: Box<dyn Write + 'a>,
    file_prefix: Option<String>,
}

impl<'a> Grapher<'a> {
    pub fn new(out: Box<dyn Write + 'a>, file_prefix: Option<&str>) -> Self {
        Self {
            out,
            file_prefix: file_prefix.map(|x| x.to_owned()),
        }
    }

    fn setup_view(sel: &DataSel, series: &DataSeries) -> (ContinuousView, f64, f64) {
        let sel_name = format!("{}", sel);

        let (vrate_max, val_max, val_min) = series
            .points
            .iter()
            .fold((0.0_f64, 0.0_f64, std::f64::MAX), |acc, point| {
                (acc.0.max(point.0), acc.1.max(point.1), acc.1.min(point.1))
            });

        let (ymin, yscale) = match sel {
            DataSel::MOF => {
                let ymin = if val_min >= 1.0 {
                    1.0
                } else {
                    val_min - (val_max - val_min) / 10.0
                };
                (ymin, 1.0)
            }
            DataSel::Lat(_, _) => (0.0, 1000.0),
        };

        let lines = &series.lines;
        let mut xlabel = format!(
            "vrate: min={:.3} max={:.3} ",
            lines.low.1 * yscale,
            lines.high.1 * yscale
        );
        if lines.low.0 > 0.0 {
            xlabel += &format!("low-infl={:.1} ", lines.low.0);
        }
        if lines.high.0 < vrate_max {
            xlabel += &format!("high-infl={:.1} ", lines.high.0);
        }
        xlabel += &format!("err={:.3}", series.error * yscale);

        let view = ContinuousView::new()
            .x_range(0.0, vrate_max * 1.1)
            .y_range(ymin * yscale, val_max * 1.1 * yscale)
            .x_label(xlabel)
            .y_label(sel_name);

        (view, vrate_max, yscale)
    }

    fn plot_one_text(&mut self, sel: &DataSel, series: &DataSeries) -> Result<()> {
        const SIZE: (u32, u32) = (80, 24);
        let (view, vrate_max, yscale) = Self::setup_view(sel, series);

        let mut lines = vec![];
        for i in 0..SIZE.0 {
            let vrate = vrate_max / SIZE.0 as f64 * i as f64;
            lines.push((vrate, series.lines.eval(vrate) * yscale));
        }
        let view =
            view.add(Plot::new(lines).point_style(PointStyle::new().marker(PointMarker::Cross)));

        let points = series
            .points
            .iter()
            .map(|(vrate, val)| (*vrate, val * yscale))
            .collect();
        let view =
            view.add(Plot::new(points).point_style(PointStyle::new().marker(PointMarker::Circle)));

        let page = Page::single(&view).dimensions(SIZE.0, SIZE.1);
        write!(self.out, "{}\n\n", page.to_text().unwrap()).unwrap();
        Ok(())
    }

    fn plot_filename(&self, sel: &DataSel) -> String {
        format!("{}-{}.svg", self.file_prefix.as_ref().unwrap(), sel)
    }

    fn plot_one_svg(&mut self, sel: &DataSel, series: &DataSeries) -> Result<()> {
        const SIZE: (u32, u32) = (640, 480);
        let (view, vrate_max, yscale) = Self::setup_view(sel, series);

        let lines = &series.lines;
        let segments = vec![
            (0.0, lines.low.1 * yscale),
            (lines.low.0, lines.low.1 * yscale),
            (lines.high.0, lines.high.1 * yscale),
            (vrate_max, lines.high.1 * yscale),
        ];
        let view = view.add(Plot::new(segments).line_style(LineStyle::new().colour("#DD3355")));

        let points = series
            .points
            .iter()
            .map(|(vrate, val)| (*vrate, val * yscale))
            .collect();
        let view = view.add(
            Plot::new(points).point_style(
                PointStyle::new()
                    .marker(PointMarker::Circle)
                    .colour("#35C788"),
            ),
        );

        let view = view.x_max_ticks(10).y_max_ticks(10);

        if let Err(e) = Page::single(&view)
            .dimensions(SIZE.0, SIZE.1)
            .save(self.plot_filename(sel))
        {
            bail!("{}", &e);
        }
        Ok(())
    }

    pub fn plot(&mut self, result: &IoCostTuneResult) -> Result<()> {
        for (sel, series) in result.data.iter() {
            self.plot_one_text(sel, series)?;
            if self.file_prefix.is_some() {
                if let Err(e) = self.plot_one_svg(sel, series) {
                    warn!(
                        "iocost-tune: Failed to plot graph into {:?} ({})",
                        &self.plot_filename(sel),
                        &e
                    );
                }
            }
        }
        Ok(())
    }
}
