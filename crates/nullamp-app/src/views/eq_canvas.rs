use crate::message::Message;
use crate::theme;

use iced::widget::canvas::{self, Frame, Path, Program, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

use nullamp_audio::eq::{EqParams, EQ_FREQUENCIES};
use std::sync::Arc;

// dB range displayed on the canvas
const DB_MAX: f32 = 12.0;
const DB_MIN: f32 = -12.0;

/// Mutable state held by the canvas across frames (drag tracking).
#[derive(Default)]
pub struct EqCanvasState {
    pub drag_band: Option<usize>,
}

/// Draggable EQ curve canvas widget.
pub struct EqCurve {
    pub eq_params: Arc<EqParams>,
    pub enabled: bool,
}

impl EqCurve {
    pub fn new(eq_params: Arc<EqParams>, enabled: bool) -> Self {
        Self { eq_params, enabled }
    }

    /// Map a canvas x coordinate to the nearest band index.
    fn x_to_band(&self, x: f32, width: f32) -> usize {
        let n = EQ_FREQUENCIES.len();
        // Log-spaced: position proportional to log(freq)
        let log_min = EQ_FREQUENCIES[0].log2();
        let log_max = EQ_FREQUENCIES[n - 1].log2();
        // Find the band whose log-position is closest
        let target_log = log_min + (x / width) * (log_max - log_min);
        let mut best = 0;
        let mut best_dist = f32::MAX;
        for (i, &freq) in EQ_FREQUENCIES.iter().enumerate() {
            let d = (freq.log2() - target_log).abs();
            if d < best_dist {
                best_dist = d;
                best = i;
            }
        }
        best
    }

    /// Map a canvas y coordinate to gain in dB.
    fn y_to_gain(&self, y: f32, height: f32) -> f32 {
        let gain = DB_MAX - (y / height) * (DB_MAX - DB_MIN);
        gain.max(DB_MIN).min(DB_MAX)
    }

    /// Map band index to canvas x position (log-scaled).
    fn band_x(&self, band: usize, width: f32) -> f32 {
        let n = EQ_FREQUENCIES.len();
        let log_min = EQ_FREQUENCIES[0].log2();
        let log_max = EQ_FREQUENCIES[n - 1].log2();
        let log_f = EQ_FREQUENCIES[band].log2();
        ((log_f - log_min) / (log_max - log_min)) * width
    }

    /// Map gain to canvas y position.
    fn gain_y(&self, gain: f32, height: f32) -> f32 {
        (DB_MAX - gain) / (DB_MAX - DB_MIN) * height
    }
}

impl Program<Message> for EqCurve {
    type State = EqCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        use canvas::Event;
        use iced::mouse;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    let band = self.x_to_band(pos.x, bounds.width);
                    state.drag_band = Some(band);
                    let gain = self.y_to_gain(pos.y, bounds.height);
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::EqBandChanged(band, gain)),
                    );
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(band) = state.drag_band {
                    if let Some(pos) = cursor.position_in(bounds) {
                        let gain = self.y_to_gain(pos.y, bounds.height);
                        return (
                            canvas::event::Status::Captured,
                            Some(Message::EqBandChanged(band, gain)),
                        );
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.drag_band = None;
            }
            _ => {}
        }
        (canvas::event::Status::Ignored, None)
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let w = bounds.width;
        let h = bounds.height;

        let mut frame = Frame::new(renderer, Size::new(w, h));

        // Background
        frame.fill_rectangle(
            Point::ORIGIN,
            Size::new(w, h),
            Color {
                a: 1.0,
                ..theme::BG_DISPLAY
            },
        );

        let alpha = if self.enabled { 1.0 } else { 0.35 };

        // Grid lines at ±12, ±6, 0 dB
        for db in [-12.0_f32, -6.0, 0.0, 6.0, 12.0] {
            let y = self.gain_y(db, h);
            let line_color = Color {
                a: if db == 0.0 {
                    0.25 * alpha
                } else {
                    0.12 * alpha
                },
                ..theme::TEXT_MUTED
            };
            let mut grid = canvas::path::Builder::new();
            grid.move_to(Point { x: 0.0, y });
            grid.line_to(Point { x: w, y });
            frame.stroke(
                &grid.build(),
                Stroke::default()
                    .with_color(line_color)
                    .with_width(if db == 0.0 { 1.0 } else { 0.5 }),
            );
        }

        // Vertical band markers (subtle)
        let n = EQ_FREQUENCIES.len();
        for i in 0..n {
            let x = self.band_x(i, w);
            let mut vline = canvas::path::Builder::new();
            vline.move_to(Point { x, y: 0.0 });
            vline.line_to(Point { x, y: h });
            frame.stroke(
                &vline.build(),
                Stroke::default()
                    .with_color(Color {
                        a: 0.07 * alpha,
                        ..theme::TEXT_MUTED
                    })
                    .with_width(0.5),
            );
        }

        // Collect band points
        let pts: Vec<Point> = (0..n)
            .map(|i| {
                let gain = self.eq_params.get_band(i);
                Point {
                    x: self.band_x(i, w),
                    y: self.gain_y(gain, h),
                }
            })
            .collect();

        // Add phantom endpoints to help the spline start/end smoothly
        let p0_extra = Point {
            x: pts[0].x - (pts[1].x - pts[0].x),
            y: pts[0].y,
        };
        let pn_extra = {
            let last = pts[n - 1];
            let prev = pts[n - 2];
            Point {
                x: last.x + (last.x - prev.x),
                y: last.y,
            }
        };
        let mut all_pts = vec![p0_extra];
        all_pts.append(&mut pts.clone());
        all_pts.push(pn_extra);

        // Build Catmull-Rom curve as cubic Bézier segments
        let mut curve = canvas::path::Builder::new();
        curve.move_to(all_pts[1]); // start at first real point

        for i in 1..all_pts.len() - 2 {
            let p0 = all_pts[i - 1];
            let p1 = all_pts[i];
            let p2 = all_pts[i + 1];
            let p3 = all_pts[i + 2];

            // Catmull-Rom → cubic Bézier control points
            let cp1 = Point {
                x: p1.x + (p2.x - p0.x) / 6.0,
                y: p1.y + (p2.y - p0.y) / 6.0,
            };
            let cp2 = Point {
                x: p2.x - (p3.x - p1.x) / 6.0,
                y: p2.y - (p3.y - p1.y) / 6.0,
            };
            curve.bezier_curve_to(cp1, cp2, p2);
        }
        let curve_path = curve.build();

        // Fill under the curve (semi-transparent green)
        let mut fill_path = canvas::path::Builder::new();
        let zero_y = self.gain_y(0.0, h);
        fill_path.move_to(Point {
            x: all_pts[1].x,
            y: zero_y,
        });
        fill_path.line_to(all_pts[1]);

        for i in 1..all_pts.len() - 2 {
            let p0 = all_pts[i - 1];
            let p1 = all_pts[i];
            let p2 = all_pts[i + 1];
            let p3 = all_pts[i + 2];
            let cp1 = Point {
                x: p1.x + (p2.x - p0.x) / 6.0,
                y: p1.y + (p2.y - p0.y) / 6.0,
            };
            let cp2 = Point {
                x: p2.x - (p3.x - p1.x) / 6.0,
                y: p2.y - (p3.y - p1.y) / 6.0,
            };
            fill_path.bezier_curve_to(cp1, cp2, p2);
        }

        // Close fill down to zero line
        let last_real = all_pts[all_pts.len() - 2];
        fill_path.line_to(Point {
            x: last_real.x,
            y: zero_y,
        });
        fill_path.close();

        frame.fill(
            &fill_path.build(),
            canvas::Fill {
                style: canvas::Style::Solid(Color {
                    r: theme::TEXT_PRIMARY.r,
                    g: theme::TEXT_PRIMARY.g,
                    b: theme::TEXT_PRIMARY.b,
                    a: 0.12 * alpha,
                }),
                ..Default::default()
            },
        );

        // Curve stroke
        frame.stroke(
            &curve_path,
            Stroke::default()
                .with_color(Color {
                    a: alpha,
                    ..theme::TEXT_PRIMARY
                })
                .with_width(1.5),
        );

        // Draw dots at each band control point
        let hover_band = cursor.position_in(bounds).map(|p| self.x_to_band(p.x, w));
        for i in 0..n {
            let gain = self.eq_params.get_band(i);
            let pt = Point {
                x: self.band_x(i, w),
                y: self.gain_y(gain, h),
            };
            let is_hover = hover_band == Some(i);
            let dot_r = if is_hover { 4.0 } else { 2.5 };
            let dot_color = if gain.abs() < 0.5 {
                Color {
                    a: 0.5 * alpha,
                    ..theme::TEXT_MUTED
                }
            } else {
                Color {
                    a: alpha,
                    ..theme::ACCENT_AMBER
                }
            };
            let dot = Path::circle(pt, dot_r);
            frame.fill(&dot, canvas::Fill::from(dot_color));
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        if state.drag_band.is_some() {
            iced::mouse::Interaction::ResizingVertically
        } else if cursor.is_over(bounds) {
            iced::mouse::Interaction::Crosshair
        } else {
            iced::mouse::Interaction::default()
        }
    }
}
