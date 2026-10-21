use crate::message::{EditorMessage, Message, ViewerMessage};
use cosmic::iced::widget::canvas::{self, Frame, Geometry, Path as CanvasPath, Program};
use cosmic::iced::widget::image::Handle;
use cosmic::iced::{keyboard, mouse};
use cosmic::iced::{Color, Point, Rectangle, Size};

pub struct ImageCanvasProgram {
    pub image_handle: Option<Handle>,
    pub img_width: f32,
    pub img_height: f32,
    pub zoom_factor: f32,
    pub is_cropping_mode: bool,
    pub is_selecting_background: bool,
    pub crop_start: Option<Point>,
    pub crop_end: Option<Point>,
}

#[derive(Debug, Clone, Copy, Default)]
enum CropDrag {
    #[default]
    None,
    New,
    Move,
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Default)]
pub struct CanvasState {
    pub is_dragging: bool,
    pub drag_start: Option<Point>,
    modifiers: keyboard::Modifiers,
    crop_drag: CropDrag,
    initial_crop: Option<(Point, Point)>,
    visual_crop: Option<(Point, Point)>,
}

impl ImageCanvasProgram {
    pub fn canvas_to_image(&self, p: Point, canvas_size: Size) -> Point {
        let zoom = self.zoom_factor;
        let img_w = self.img_width;
        let img_h = self.img_height;
        let rendered_w = img_w * zoom;
        let rendered_h = img_h * zoom;
        let rendered_x = (canvas_size.width - rendered_w) / 2.0;
        let rendered_y = (canvas_size.height - rendered_h) / 2.0;

        Point::new(
            ((p.x - rendered_x) / zoom).clamp(0.0, img_w),
            ((p.y - rendered_y) / zoom).clamp(0.0, img_h),
        )
    }

    pub fn image_to_canvas(&self, p: Point, canvas_size: Size) -> Point {
        let zoom = self.zoom_factor;
        let img_w = self.img_width;
        let img_h = self.img_height;
        let rendered_w = img_w * zoom;
        let rendered_h = img_h * zoom;
        let rendered_x = (canvas_size.width - rendered_w) / 2.0;
        let rendered_y = (canvas_size.height - rendered_h) / 2.0;

        Point::new(p.x * zoom + rendered_x, p.y * zoom + rendered_y)
    }
}

impl Program<Message, cosmic::theme::Theme, cosmic::iced::Renderer> for ImageCanvasProgram {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let canvas_size = bounds.size();

        let canvas_to_image = |p: Point| -> Point { self.canvas_to_image(p, canvas_size) };

        match event {
            canvas::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta })
                if state.modifiers.control() =>
            {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                        *y
                    }
                };

                let message = if y > 0.0 {
                    Some(ViewerMessage::ZoomIn)
                } else if y < 0.0 {
                    Some(ViewerMessage::ZoomOut)
                } else {
                    None
                };

                if let Some(message) = message {
                    return Some(canvas::Action::publish(Message::Viewer(message)).and_capture());
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    if self.is_selecting_background {
                        let rendered_w = self.img_width * self.zoom_factor;
                        let rendered_h = self.img_height * self.zoom_factor;
                        let rendered_x = (canvas_size.width - rendered_w) / 2.0;
                        let rendered_y = (canvas_size.height - rendered_h) / 2.0;
                        if pos.x >= rendered_x
                            && pos.x < rendered_x + rendered_w
                            && pos.y >= rendered_y
                            && pos.y < rendered_y + rendered_h
                        {
                            let image_point = canvas_to_image(pos);
                            return Some(
                                canvas::Action::publish(Message::Editor(
                                    EditorMessage::BackgroundSeedPicked(image_point),
                                ))
                                .and_capture(),
                            );
                        }
                        return Some(canvas::Action::capture());
                    }
                    state.is_dragging = true;
                    state.drag_start = Some(pos);

                    if self.is_cropping_mode {
                        if let (Some(a), Some(b)) = (self.crop_start, self.crop_end) {
                            let min = Point::new(a.x.min(b.x), a.y.min(b.y));
                            let max = Point::new(a.x.max(b.x), a.y.max(b.y));
                            let p1 = self.image_to_canvas(min, canvas_size);
                            let p2 = self.image_to_canvas(max, canvas_size);
                            let tolerance = 10.0;
                            let near_l = (pos.x - p1.x).abs() <= tolerance;
                            let near_r = (pos.x - p2.x).abs() <= tolerance;
                            let near_t = (pos.y - p1.y).abs() <= tolerance;
                            let near_b = (pos.y - p2.y).abs() <= tolerance;
                            let inside_x = pos.x >= p1.x && pos.x <= p2.x;
                            let inside_y = pos.y >= p1.y && pos.y <= p2.y;

                            state.crop_drag = match (near_l, near_r, near_t, near_b) {
                                (true, _, true, _) => CropDrag::TopLeft,
                                (_, true, true, _) => CropDrag::TopRight,
                                (true, _, _, true) => CropDrag::BottomLeft,
                                (_, true, _, true) => CropDrag::BottomRight,
                                (true, _, _, _) if inside_y => CropDrag::Left,
                                (_, true, _, _) if inside_y => CropDrag::Right,
                                (_, _, true, _) if inside_x => CropDrag::Top,
                                (_, _, _, true) if inside_x => CropDrag::Bottom,
                                _ if inside_x && inside_y => CropDrag::Move,
                                _ => CropDrag::New,
                            };
                            state.initial_crop = Some((min, max));
                            state.visual_crop = Some((min, max));
                        } else {
                            state.crop_drag = CropDrag::New;
                            state.initial_crop = None;
                            let start = canvas_to_image(pos);
                            state.visual_crop = Some((start, start));
                        }

                        let start_img = canvas_to_image(pos);
                        if matches!(state.crop_drag, CropDrag::New) {
                            return Some(
                                canvas::Action::publish(Message::Editor(EditorMessage::CropStart(
                                    start_img,
                                )))
                                .and_capture(),
                            );
                        }
                        return Some(canvas::Action::capture());
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.is_dragging {
                    if let Some(pos) = cursor.position_in(bounds) {
                        if self.is_cropping_mode {
                            let current = canvas_to_image(pos);
                            if matches!(state.crop_drag, CropDrag::New) {
                                if let Some((start, _)) = state.visual_crop {
                                    state.visual_crop = Some((start, current));
                                }
                                return Some(
                                    canvas::Action::publish(Message::Editor(
                                        EditorMessage::CropUpdate(current),
                                    ))
                                    .and_capture(),
                                );
                            }
                            if let (Some(start_canvas), Some((mut min, mut max))) =
                                (state.drag_start, state.initial_crop)
                            {
                                let origin = canvas_to_image(start_canvas);
                                match state.crop_drag {
                                    CropDrag::Move => {
                                        let width = max.x - min.x;
                                        let height = max.y - min.y;
                                        let dx = (current.x - origin.x)
                                            .clamp(-min.x, self.img_width - max.x);
                                        let dy = (current.y - origin.y)
                                            .clamp(-min.y, self.img_height - max.y);
                                        min.x += dx;
                                        max.x = min.x + width;
                                        min.y += dy;
                                        max.y = min.y + height;
                                    }
                                    CropDrag::Left | CropDrag::TopLeft | CropDrag::BottomLeft => {
                                        min.x = current.x.min(max.x)
                                    }
                                    CropDrag::Right
                                    | CropDrag::TopRight
                                    | CropDrag::BottomRight => max.x = current.x.max(min.x),
                                    _ => {}
                                }
                                match state.crop_drag {
                                    CropDrag::Top | CropDrag::TopLeft | CropDrag::TopRight => {
                                        min.y = current.y.min(max.y)
                                    }
                                    CropDrag::Bottom
                                    | CropDrag::BottomLeft
                                    | CropDrag::BottomRight => max.y = current.y.max(min.y),
                                    _ => {}
                                }
                                state.visual_crop = Some((min, max));
                                return Some(
                                    canvas::Action::publish(Message::Editor(
                                        EditorMessage::SetCrop(min, max),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_dragging {
                    let was_new_selection = matches!(state.crop_drag, CropDrag::New);
                    state.is_dragging = false;
                    state.crop_drag = CropDrag::None;
                    state.initial_crop = None;
                    if self.is_cropping_mode && was_new_selection {
                        if let Some(pos) = cursor.position_in(bounds) {
                            let end_img = canvas_to_image(pos);
                            return Some(
                                canvas::Action::publish(Message::Editor(EditorMessage::CropEnd(
                                    end_img,
                                )))
                                .and_capture(),
                            );
                        }
                    }
                    if self.is_cropping_mode {
                        return Some(canvas::Action::capture());
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if self.is_selecting_background && cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &cosmic::iced::Renderer,
        _theme: &cosmic::theme::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let canvas_size = bounds.size();

        if self.image_handle.is_some() {
            let zoom = self.zoom_factor;
            let img_w = self.img_width;
            let img_h = self.img_height;
            let rendered_w = img_w * zoom;
            let rendered_h = img_h * zoom;
            let rendered_x = (canvas_size.width - rendered_w) / 2.0;
            let rendered_y = (canvas_size.height - rendered_h) / 2.0;

            let crop = self
                .crop_start
                .zip(self.crop_end)
                .map(|app_crop| state.visual_crop.unwrap_or(app_crop));
            if self.is_cropping_mode {
                if let Some((c_start, c_end)) = crop {
                    let p1 = self.image_to_canvas(c_start, canvas_size);
                    let p2 = self.image_to_canvas(c_end, canvas_size);

                    let min_x = p1.x.min(p2.x).max(rendered_x).min(rendered_x + rendered_w);
                    let max_x = p1.x.max(p2.x).max(rendered_x).min(rendered_x + rendered_w);
                    let min_y = p1.y.min(p2.y).max(rendered_y).min(rendered_y + rendered_h);
                    let max_y = p1.y.max(p2.y).max(rendered_y).min(rendered_y + rendered_h);

                    let crop_rect = Rectangle::new(
                        Point::new(min_x, min_y),
                        Size::new(max_x - min_x, max_y - min_y),
                    );

                    let top_mask = Rectangle::new(
                        Point::new(rendered_x, rendered_y),
                        Size::new(rendered_w, (min_y - rendered_y).max(0.0)),
                    );
                    frame.fill(
                        &CanvasPath::rectangle(top_mask.position(), top_mask.size()),
                        Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    );

                    let bottom_mask = Rectangle::new(
                        Point::new(rendered_x, max_y),
                        Size::new(rendered_w, (rendered_y + rendered_h - max_y).max(0.0)),
                    );
                    frame.fill(
                        &CanvasPath::rectangle(bottom_mask.position(), bottom_mask.size()),
                        Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    );

                    let left_mask = Rectangle::new(
                        Point::new(rendered_x, min_y),
                        Size::new((min_x - rendered_x).max(0.0), max_y - min_y),
                    );
                    frame.fill(
                        &CanvasPath::rectangle(left_mask.position(), left_mask.size()),
                        Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    );

                    let right_mask = Rectangle::new(
                        Point::new(max_x, min_y),
                        Size::new((rendered_x + rendered_w - max_x).max(0.0), max_y - min_y),
                    );
                    frame.fill(
                        &CanvasPath::rectangle(right_mask.position(), right_mask.size()),
                        Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                    );

                    if crop_rect.width > 0.0 && crop_rect.height > 0.0 {
                        let path = CanvasPath::rectangle(crop_rect.position(), crop_rect.size());
                        frame.stroke(
                            &path,
                            canvas::Stroke::default()
                                .with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.85))
                                .with_width(6.0),
                        );
                        frame.stroke(
                            &path,
                            canvas::Stroke::default()
                                .with_color(Color::from_rgb(0.20, 0.78, 1.0))
                                .with_width(3.0),
                        );

                        // Visible resize handles on every corner and edge midpoint.
                        let handle_size = 8.0;
                        let half = handle_size / 2.0;
                        let points = [
                            Point::new(min_x, min_y),
                            Point::new((min_x + max_x) / 2.0, min_y),
                            Point::new(max_x, min_y),
                            Point::new(min_x, (min_y + max_y) / 2.0),
                            Point::new(max_x, (min_y + max_y) / 2.0),
                            Point::new(min_x, max_y),
                            Point::new((min_x + max_x) / 2.0, max_y),
                            Point::new(max_x, max_y),
                        ];
                        for point in points {
                            let handle_back = Rectangle::new(
                                Point::new(point.x - half - 2.0, point.y - half - 2.0),
                                Size::new(handle_size + 4.0, handle_size + 4.0),
                            );
                            frame.fill(
                                &CanvasPath::rectangle(handle_back.position(), handle_back.size()),
                                Color::from_rgba(0.0, 0.0, 0.0, 0.85),
                            );
                            let handle = Rectangle::new(
                                Point::new(point.x - half, point.y - half),
                                Size::new(handle_size, handle_size),
                            );
                            frame.fill(
                                &CanvasPath::rectangle(handle.position(), handle.size()),
                                Color::from_rgb(0.20, 0.78, 1.0),
                            );
                        }
                    }
                }
            }
        }

        vec![frame.into_geometry()]
    }
}
