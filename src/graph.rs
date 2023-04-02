use druid::widget::prelude::*;
use druid::{Data, Lens, Color, Rect};
use druid::kurbo::{BezPath, Point, Circle};

#[derive(Clone, Data, Lens)]
pub struct GraphData {
    // values between 0.0 and 1.0
    #[data(eq)]
    pub data: Vec<f64>,

    // value between 0.0 and 1.0
    pub min_line: f64,

    #[data(eq)]
    pub peaks: Vec<usize>,

    pub min_range: f64,
    pub max_range: f64,
}

impl GraphData {
    #[inline]
    pub fn value_to_pixel(height: f64, value: f64) -> f64 {
        height - value * height
    }
}

// a custom widget that draws a line graph
pub struct LineGraph;

impl LineGraph {
    // create a new instance of the widget
    fn new() -> Self {
        Self
    }
}

impl Widget<GraphData> for LineGraph {
    fn event(&mut self, _ctx: &mut EventCtx, event: &Event, _data: &mut GraphData, _env: &Env) {
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &GraphData, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &GraphData, _data: &GraphData, _env: &Env) {
        // request a repaint when the data changes
        ctx.request_paint();
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &GraphData, _env: &Env) -> Size {
        // use the maximum size available
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &GraphData, env: &Env) {
        // get the size of the widget
        let size = ctx.size();

        // create a color for the line graph
        let color = Color::rgb8(0x1e, 0xcb, 0xe1);

        let grey = Color::grey(0.8);
        // create a path for the line graph
        let mut path = BezPath::new();

        // move to the first point of the line graph
        if let Some(first) = data.data.first() {
            path.move_to(Point::new(0.0, GraphData::value_to_pixel(size.height, *first)));
        }

        // add lines to the rest of the points of the line graph
        for (i, value) in data.data.iter().enumerate().skip(1) {
            path.line_to(Point::new(i as f64 * size.width / (data.data.len() - 1) as f64, GraphData::value_to_pixel(size.height, *value)));
        }

        // stroke the path with some thickness
        ctx.stroke(path, &color, 2.0);

        let mut path = BezPath::new();
        path.move_to(Point::new(0.0, GraphData::value_to_pixel(size.height, data.min_line)));
        path.line_to(Point::new(size.width, GraphData::value_to_pixel(size.height, data.min_line)));
        ctx.stroke(path, &grey, 2.0);

        for peak in &data.peaks {
            let circle = Circle::new(Point::new(*peak as f64 * size.width / (data.data.len() - 1) as f64, GraphData::value_to_pixel(size.height, data.data[*peak])), 5.0);
            ctx.fill(circle, &Color::rgb8(255, 255, 255))
        }

        let mut path = BezPath::new();
        path.move_to(Point::new(data.min_range * size.width, 0.0));
        path.line_to(Point::new(data.min_range * size.width, size.height));
        ctx.stroke(path, &grey, 2.0);

        let mut path = BezPath::new();
        path.move_to(Point::new(data.max_range * size.width, 0.0));
        path.line_to(Point::new(data.max_range * size.width, size.height));
        ctx.stroke(path, &grey, 2.0);

        let selected = Rect::new( data.min_range * size.width, 0.0, data.max_range * size.width, GraphData::value_to_pixel(size.height, data.min_line));
        ctx.fill(selected, &Color::rgba8(255, 255, 255, 50));
    }
}