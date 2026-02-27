use iced::widget::canvas::{self, Action, Cache, Canvas, Event, Geometry, Path, Program};
use iced::widget::{
    Container, Scrollable, Space, button, checkbox, column, container, pick_list, row, text,
    text_input,
};
use iced::{
    Color, Element, Length, Point, Rectangle, Renderer, Size, Task, Theme, alignment, mouse,
};
use std::str::FromStr;

use crate::backend::{Output, OutputMode, wlr_randr_apply, wlr_randr_get_outputs, wlr_randr_restore_default, wlr_randr_save};

#[derive(Debug, Clone)]
pub enum Message {
    MonitorClicked(usize),
    MonitorPositioned(usize, i32, i32),
    XChanged(String),
    YChanged(String),
    XInc,
    XDec,
    YInc,
    YDec,
    ScaleChanged(String),
    ScaleInc,
    ScaleDec,
    EnabledToggled(bool),
    ResolutionSelected(usize),
    TransformSelected(String),
    ApplyClicked,
    SaveClicked,
    RestoreDefaultClicked,
}

pub struct MangoDisplay {
    outputs: Vec<Output>,
    selected_output_idx: Option<usize>,
    layout_cache: Cache,
    x_input: String,
    y_input: String,
    scale_input: String,
    pub settings: crate::settings::AppSettings,
    status_message: Option<String>,
}

impl Default for MangoDisplay {
    fn default() -> Self {
        let outputs = wlr_randr_get_outputs().unwrap_or_default();
        let selected_output_idx = if !outputs.is_empty() { Some(0) } else { None };
        let mut app = Self {
            outputs,
            selected_output_idx,
            layout_cache: Cache::default(),
            x_input: String::new(),
            y_input: String::new(),
            scale_input: String::new(),
            settings: crate::settings::AppSettings::load(),
            status_message: None,
        };
        app.update_inputs_for_selection();
        app
    }
}

impl MangoDisplay {
    fn update_inputs_for_selection(&mut self) {
        if let Some(idx) = self.selected_output_idx {
            let out = &self.outputs[idx];
            self.x_input = out.position.0.to_string();
            self.y_input = out.position.1.to_string();
            self.scale_input = format!("{:.2}", out.scale);
        }
    }

    fn normalize_positions(&mut self) {
        let min_x = self.outputs.iter().map(|o| o.position.0).min().unwrap_or(0);
        let min_y = self.outputs.iter().map(|o| o.position.1).min().unwrap_or(0);

        let mut changed = false;
        let offset_x = if min_x < 0 { -min_x } else { 0 };
        let offset_y = if min_y < 0 { -min_y } else { 0 };

        if offset_x > 0 || offset_y > 0 {
            for out in &mut self.outputs {
                out.position.0 += offset_x;
                out.position.1 += offset_y;
            }
            changed = true;
        }

        if changed {
            self.update_inputs_for_selection();
            self.layout_cache.clear();
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MonitorClicked(idx) => {
                self.selected_output_idx = Some(idx);
                self.update_inputs_for_selection();
                self.layout_cache.clear();
            }
            Message::MonitorPositioned(idx, x, y) => {
                self.outputs[idx].position = (x, y);
                if Some(idx) == self.selected_output_idx {
                    self.update_inputs_for_selection();
                }
                self.layout_cache.clear();
            }
            Message::XChanged(val) => {
                self.x_input = val.clone();
                if let (Some(idx), Ok(mut v)) = (self.selected_output_idx, i32::from_str(&val)) {
                    if v < 0 {
                        v = 0;
                    }
                    self.outputs[idx].position.0 = v;
                    self.layout_cache.clear();
                }
            }
            Message::YChanged(val) => {
                self.y_input = val.clone();
                if let (Some(idx), Ok(mut v)) = (self.selected_output_idx, i32::from_str(&val)) {
                    if v < 0 {
                        v = 0;
                    }
                    self.outputs[idx].position.1 = v;
                    self.layout_cache.clear();
                }
            }
            Message::XInc => {
                if let Some(idx) = self.selected_output_idx {
                    self.outputs[idx].position.0 += 1;
                    self.update_inputs_for_selection();
                    self.layout_cache.clear();
                }
            }
            Message::XDec => {
                if let Some(idx) = self.selected_output_idx {
                    if self.outputs[idx].position.0 > 0 {
                        self.outputs[idx].position.0 -= 1;
                        self.update_inputs_for_selection();
                        self.layout_cache.clear();
                    }
                }
            }
            Message::YInc => {
                if let Some(idx) = self.selected_output_idx {
                    self.outputs[idx].position.1 += 1;
                    self.update_inputs_for_selection();
                    self.layout_cache.clear();
                }
            }
            Message::YDec => {
                if let Some(idx) = self.selected_output_idx {
                    if self.outputs[idx].position.1 > 0 {
                        self.outputs[idx].position.1 -= 1;
                        self.update_inputs_for_selection();
                        self.layout_cache.clear();
                    }
                }
            }
            Message::ScaleChanged(val) => {
                self.scale_input = val.clone();
                if let (Some(idx), Ok(v)) = (self.selected_output_idx, f32::from_str(&val)) {
                    if v > 0.1 {
                        self.outputs[idx].scale = v;
                        self.layout_cache.clear();
                    }
                }
            }
            Message::ScaleInc => {
                if let Some(idx) = self.selected_output_idx {
                    self.outputs[idx].scale += 0.05;
                    self.update_inputs_for_selection();
                    self.layout_cache.clear();
                }
            }
            Message::ScaleDec => {}
            Message::EnabledToggled(val) => {
                if let Some(idx) = self.selected_output_idx {
                    self.outputs[idx].enabled = val;
                    self.layout_cache.clear();
                }
            }
            Message::ResolutionSelected(res_idx) => {
                if let Some(idx) = self.selected_output_idx {
                    for m in &mut self.outputs[idx].modes {
                        m.current = false;
                    }
                    if res_idx < self.outputs[idx].modes.len() {
                        self.outputs[idx].modes[res_idx].current = true;
                    }
                    self.layout_cache.clear();
                }
            }
            Message::TransformSelected(trans) => {
                if let Some(idx) = self.selected_output_idx {
                    self.outputs[idx].transform = trans;
                    self.layout_cache.clear();
                }
            }
            Message::ApplyClicked => {
                self.normalize_positions();
                match wlr_randr_apply(&self.outputs) {
                    Ok(()) => self.status_message = Some("Applied successfully!".to_string()),
                    Err(e) => self.status_message = Some(format!("Apply error: {}", e)),
                }
            }
            Message::SaveClicked => {
                self.normalize_positions();
                match wlr_randr_save(&self.outputs, &self.settings) {
                    Ok(()) => {
                        self.status_message =
                            Some(format!("Saved to {}", self.settings.monitors_conf_path))
                    }
                    Err(e) => self.status_message = Some(format!("Save error: {}", e)),
                }
            }
            Message::RestoreDefaultClicked => {
                match wlr_randr_restore_default(&self.settings) {
                    Ok(()) => self.status_message = Some("Restored to default config!".to_string()),
                    Err(e) => self.status_message = Some(format!("Restore error: {}", e)),
                }
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let canvas = Canvas::new(LayoutCanvas {
            outputs: self.outputs.clone(),
            selected_idx: self.selected_output_idx,
            cache: &self.layout_cache,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        let mut sidebar = column![].spacing(15).width(Length::Fixed(400.0));

        let mut tabs_row = row![].spacing(0);
        for (i, out) in self.outputs.iter().enumerate() {
            let is_selected = Some(i) == self.selected_output_idx;
            let current_btn = button(text(&out.name).align_x(alignment::Horizontal::Center))
                .width(Length::Fixed(80.0))
                .style(if is_selected {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::MonitorClicked(i));
            tabs_row = tabs_row.push(current_btn);
        }
        sidebar = sidebar.push(container(tabs_row).center_x(Length::Fill));

        if let Some(idx) = self.selected_output_idx {
            let out = &self.outputs[idx];

            if self.outputs.len() > 1 {
                sidebar = sidebar.push(
                    row![
                        Space::new().width(100.0),
                        checkbox(out.enabled).on_toggle(Message::EnabledToggled),
                        text("Enabled")
                    ]
                    .spacing(10),
                );
            }

            let label_width = 100.0;

            let row_desc = row![
                container(text("Description").size(14)).width(label_width),
                text(&out.description).size(14)
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center);
            sidebar = sidebar.push(row_desc);

            let phys_size_text = if out.physical_size.is_empty() {
                "Unknown".to_string()
            } else {
                out.physical_size.clone()
            };
            let row_phys = row![
                container(text("Physical Size").size(14)).width(label_width),
                text(phys_size_text).size(14)
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center);
            sidebar = sidebar.push(row_phys);

            let row_scale = row![
                container(text("DPI Scale").size(14)).width(label_width),
                text_input("", &self.scale_input)
                    .on_input(Message::ScaleChanged)
                    .width(Length::Fixed(60.0)),
                button("-").on_press(Message::ScaleDec),
                button("+").on_press(Message::ScaleInc),
            ]
            .spacing(5)
            .align_y(alignment::Vertical::Center);
            sidebar = sidebar.push(row_scale);

            let row_pos = row![
                container(text("Position").size(14)).width(label_width),
                text_input("", &self.x_input)
                    .on_input(Message::XChanged)
                    .width(Length::Fixed(60.0)),
                button("-").on_press(Message::XDec),
                button("+").on_press(Message::XInc),
                text_input("", &self.y_input)
                    .on_input(Message::YChanged)
                    .width(Length::Fixed(60.0)),
                button("-").on_press(Message::YDec),
                button("+").on_press(Message::YInc),
            ]
            .spacing(5)
            .align_y(alignment::Vertical::Center);
            sidebar = sidebar.push(row_pos);

            let cm = out
                .modes
                .iter()
                .find(|m| m.current)
                .cloned()
                .unwrap_or(OutputMode {
                    width: 1920,
                    height: 1080,
                    refresh_rate: 60.0,
                    current: true,
                    preferred: false,
                });

            let row_size = row![
                container(text("Size").size(14)).width(label_width),
                text_input("", &cm.width.to_string()).width(Length::Fixed(60.0)),
                button("-"),
                button("+"),
                text("x"),
                text_input("", &cm.height.to_string()).width(Length::Fixed(60.0)),
                button("-"),
                button("+"),
                button("!"),
            ]
            .spacing(5)
            .align_y(alignment::Vertical::Center);
            sidebar = sidebar.push(row_size);

            let mut current_res_idx = 0;
            let mut resolutions = Vec::new();
            for (i, m) in out.modes.iter().enumerate() {
                resolutions.push(format!("{:.3}", m.refresh_rate));
                if m.current {
                    current_res_idx = i;
                }
            }
            let res_options = resolutions.clone();
            let selected_res = if current_res_idx < resolutions.len() {
                Some(resolutions[current_res_idx].clone())
            } else {
                None
            };
            let pick_rr = pick_list(res_options, selected_res, move |selected: String| {
                let i = resolutions.iter().position(|r| *r == selected).unwrap_or(0);
                Message::ResolutionSelected(i)
            })
            .width(Length::Fixed(100.0));

            let row_rr = row![
                container(text("Refresh Rate").size(14)).width(label_width),
                pick_rr,
                button("-"),
                button("+"),
                text("Hz").size(14)
            ]
            .spacing(5)
            .align_y(alignment::Vertical::Center);
            sidebar = sidebar.push(row_rr);

            let transforms = vec![
                "normal".to_string(),
                "90".to_string(),
                "180".to_string(),
                "270".to_string(),
                "flipped".to_string(),
                "flipped-90".to_string(),
                "flipped-180".to_string(),
                "flipped-270".to_string(),
            ];
            let pick_trans = pick_list(transforms.clone(), Some(out.transform.clone()), |t| {
                Message::TransformSelected(t)
            })
            .width(Length::Fixed(200.0));

            let row_trans = row![
                container(text("Transform").size(14)).width(label_width),
                pick_trans
            ]
            .spacing(5)
            .align_y(alignment::Vertical::Center);
            sidebar = sidebar.push(row_trans);
        }

        if let Some(ref msg) = self.status_message {
            sidebar = sidebar.push(text(msg).size(13));
        }

        let actions = row![
            button("Apply").on_press(Message::ApplyClicked),
            button("Save").on_press(Message::SaveClicked),
            button("Restore Default").on_press(Message::RestoreDefaultClicked),
        ]
        .spacing(10);

        sidebar = sidebar.push(Space::new().width(0.0).height(Length::Fill));
        sidebar = sidebar.push(actions);

        let main_content = row![
            Container::new(canvas)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(container::dark),
            Container::new(Scrollable::new(sidebar).height(Length::Fill))
                .padding(20)
                .style(container::dark)
        ];

        main_content.into()
    }
}

#[derive(Default)]
pub struct CanvasState {
    dragging: Option<(usize, Point, (i32, i32))>,
    hovered: Option<usize>,
}

struct LayoutCanvas<'a> {
    outputs: Vec<Output>,
    selected_idx: Option<usize>,
    cache: &'a Cache,
}

impl<'a> LayoutCanvas<'a> {
    fn logical_size(out: &Output, cm: &OutputMode) -> (i32, i32) {
        let w = (cm.width as f32 / out.scale) as i32;
        let h = (cm.height as f32 / out.scale) as i32;
        match out.transform.as_str() {
            "90" | "270" | "flipped-90" | "flipped-270" => (h, w),
            _ => (w, h),
        }
    }

    fn calculate_layout(&self, bounds: Rectangle) -> (f32, f32, f32, i32, i32) {
        let mut total_w = 0;
        let mut max_h = 1080;

        for out in &self.outputs {
            let cm = out
                .modes
                .iter()
                .find(|m| m.current)
                .cloned()
                .unwrap_or(OutputMode {
                    width: 800,
                    height: 600,
                    refresh_rate: 60.0,
                    current: true,
                    preferred: false,
                });
            let (w_logical, h_logical) = Self::logical_size(out, &cm);
            total_w += w_logical;
            if h_logical > max_h {
                max_h = h_logical;
            }
        }

        let span_x = (total_w as f32 * 1.5).max(4000.0);
        let span_y = (max_h as f32 * 2.5).max(3000.0);
        let scale = (bounds.width / span_x).min(bounds.height / span_y);

        let first_w = self
            .outputs
            .first()
            .map(|out| {
                let cm = out
                    .modes
                    .iter()
                    .find(|m| m.current)
                    .cloned()
                    .unwrap_or(OutputMode {
                        width: 800,
                        height: 600,
                        refresh_rate: 60.0,
                        current: true,
                        preferred: false,
                    });
                Self::logical_size(out, &cm).0 as f32
            })
            .unwrap_or(1920.0);

        let offset_x = bounds.width / 2.0 - (first_w / 2.0) * scale;
        let offset_y = bounds.height / 2.0 - (max_h as f32 / 2.0) * scale;

        (scale, offset_x, offset_y, 0, 0)
    }

    fn transformed_geometry(
        &self,
        out: &Output,
        scale: f32,
        offset_x: f32,
        offset_y: f32,
        min_x: i32,
        min_y: i32,
    ) -> (f32, f32, f32, f32) {
        let cm = out
            .modes
            .iter()
            .find(|m| m.current)
            .cloned()
            .unwrap_or(OutputMode {
                width: 800,
                height: 600,
                refresh_rate: 60.0,
                current: true,
                preferred: false,
            });
        let (w_logical, h_logical) = Self::logical_size(out, &cm);

        let w = w_logical as f32 * scale;
        let h = h_logical as f32 * scale;
        let x = (out.position.0 - min_x) as f32 * scale + offset_x;
        let y = (out.position.1 - min_y) as f32 * scale + offset_y;

        (x, y, w, h)
    }
}

impl<'a> Program<Message> for LayoutCanvas<'a> {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let (scale, offset_x, offset_y, min_x, min_y) = self.calculate_layout(bounds);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(cursor_position) = cursor.position_in(bounds) {
                    for (i, out) in self.outputs.iter().enumerate() {
                        let (x, y, w, h) =
                            self.transformed_geometry(out, scale, offset_x, offset_y, min_x, min_y);

                        let rect = Rectangle::new(Point::new(x, y), Size::new(w, h));
                        if rect.contains(cursor_position) {
                            state.dragging = Some((i, cursor_position, out.position));
                        }
                    }
                    if let Some((i, _, _)) = state.dragging {
                        return Some(Action::publish(Message::MonitorClicked(i)).and_capture());
                    }
                    state.dragging = None;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging = None;
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some((idx, start_cursor, start_logical)) = state.dragging {
                    let delta_x = (position.x - start_cursor.x) / scale;
                    let delta_y = (position.y - start_cursor.y) / scale;

                    let mut new_x = start_logical.0 + delta_x.round() as i32;
                    let mut new_y = start_logical.1 + delta_y.round() as i32;
                    if new_x < 0 {
                        new_x = 0;
                    }
                    if new_y < 0 {
                        new_y = 0;
                    }

                    let snap_threshold = 40;

                    let out = &self.outputs[idx];
                    let cm = out
                        .modes
                        .iter()
                        .find(|m| m.current)
                        .cloned()
                        .unwrap_or(OutputMode {
                            width: 800,
                            height: 600,
                            refresh_rate: 60.0,
                            current: true,
                            preferred: false,
                        });
                    let (w, h) = Self::logical_size(out, &cm);

                    let mut snapped_x = new_x;
                    let mut snapped_y = new_y;
                    let mut min_dist_x = snap_threshold;
                    let mut min_dist_y = snap_threshold;

                    let my_left = new_x;
                    let my_right = new_x + w;
                    let my_top = new_y;
                    let my_bottom = new_y + h;

                    for (i, other) in self.outputs.iter().enumerate() {
                        if i == idx {
                            continue;
                        }
                        let other_cm =
                            other
                                .modes
                                .iter()
                                .find(|m| m.current)
                                .cloned()
                                .unwrap_or(OutputMode {
                                    width: 800,
                                    height: 600,
                                    refresh_rate: 60.0,
                                    current: true,
                                    preferred: false,
                                });
                        let (other_w, other_h) = Self::logical_size(other, &other_cm);

                        let other_left = other.position.0;
                        let other_right = other.position.0 + other_w;
                        let other_top = other.position.1;
                        let other_bottom = other.position.1 + other_h;

                        let x_overlap = my_left < other_right + snap_threshold
                            && my_right > other_left - snap_threshold;
                        let y_overlap = my_top < other_bottom + snap_threshold
                            && my_bottom > other_top - snap_threshold;

                        if y_overlap {
                            if (my_left - other_right).abs() < min_dist_x {
                                min_dist_x = (my_left - other_right).abs();
                                snapped_x = other_right;
                            }
                            if (my_right - other_left).abs() < min_dist_x {
                                min_dist_x = (my_right - other_left).abs();
                                snapped_x = other_left - w;
                            }
                            if (my_left - other_left).abs() < min_dist_x {
                                min_dist_x = (my_left - other_left).abs();
                                snapped_x = other_left;
                            }
                        }

                        if x_overlap {
                            if (my_top - other_bottom).abs() < min_dist_y {
                                min_dist_y = (my_top - other_bottom).abs();
                                snapped_y = other_bottom;
                            }
                            if (my_bottom - other_top).abs() < min_dist_y {
                                min_dist_y = (my_bottom - other_top).abs();
                                snapped_y = other_top - h;
                            }
                            if (my_top - other_top).abs() < min_dist_y {
                                min_dist_y = (my_top - other_top).abs();
                                snapped_y = other_top;
                            }
                        }
                    }

                    if snapped_x == new_x {
                        snapped_x = (snapped_x as f32 / 10.0).round() as i32 * 10;
                    }
                    if snapped_y == new_y {
                        snapped_y = (snapped_y as f32 / 10.0).round() as i32 * 10;
                    }

                    if snapped_x < 0 {
                        snapped_x = 0;
                    }
                    if snapped_y < 0 {
                        snapped_y = 0;
                    }

                    return Some(Action::publish(Message::MonitorPositioned(
                        idx, snapped_x, snapped_y,
                    )));
                } else {
                    let mut new_hovered = None;
                    for (i, out) in self.outputs.iter().enumerate() {
                        let (x, y, w, h) =
                            self.transformed_geometry(out, scale, offset_x, offset_y, min_x, min_y);
                        let rect = Rectangle::new(Point::new(x, y), Size::new(w, h));
                        if rect.contains(*position) {
                            new_hovered = Some(i);
                        }
                    }
                    if state.hovered != new_hovered {
                        state.hovered = new_hovered;
                        self.cache.clear();
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(15, 15, 15));

            let (scale, offset_x, offset_y, min_x, min_y) = self.calculate_layout(bounds);

            for (i, out) in self.outputs.iter().enumerate() {
                let (x, y, w, h) =
                    self.transformed_geometry(out, scale, offset_x, offset_y, min_x, min_y);

                let rect = Rectangle::new(Point::new(x, y), Size::new(w, h));

                let is_selected = Some(i) == self.selected_idx;
                let is_hovered = Some(i) == state.hovered;

                let fill_color = if is_selected {
                    Color::from_rgb8(220, 220, 220)
                } else if is_hovered {
                    Color::from_rgb8(60, 60, 60)
                } else {
                    Color::from_rgb8(35, 35, 35)
                };

                let stroke_color = if is_selected {
                    Color::from_rgb8(255, 255, 255)
                } else if is_hovered {
                    Color::from_rgb8(150, 150, 150)
                } else {
                    Color::from_rgb8(20, 20, 20)
                };

                frame.fill_rectangle(rect.position(), rect.size(), fill_color);

                frame.stroke(
                    &Path::rectangle(rect.position(), rect.size()),
                    canvas::Stroke::default()
                        .with_color(stroke_color)
                        .with_width(if is_selected { 3.0 } else { 2.0 }),
                );

                let text_x = x + 16.0;
                let mut text_y = y + 16.0;
                let font_scale = scale.min(2.0).max(0.5);

                let mut name_text = canvas::Text::default();
                name_text.content = out.name.clone();
                name_text.position = Point::new(text_x, text_y);
                name_text.size = iced::Pixels(48.0 * font_scale);
                name_text.color = if is_selected {
                    Color::BLACK
                } else {
                    Color::from_rgb8(230, 230, 230)
                };
                frame.fill_text(name_text);

                text_y += 50.0 * font_scale;

                let text_size = 18.0 * font_scale;
                let approx_char_width = text_size * 0.6;
                let max_chars = ((w - 32.0) / approx_char_width).max(10.0) as usize;

                let mut lines = Vec::new();
                let mut current_line = String::new();

                for word in out.description.split_whitespace() {
                    if current_line.len() + word.len() + 1 > max_chars && !current_line.is_empty() {
                        lines.push(current_line);
                        current_line = word.to_string();
                    } else {
                        if !current_line.is_empty() {
                            current_line.push(' ');
                        }
                        current_line.push_str(word);
                    }
                }
                if !current_line.is_empty() {
                    lines.push(current_line);
                }

                for line in lines {
                    let mut desc_text = canvas::Text::default();
                    desc_text.content = line;
                    desc_text.position = Point::new(text_x, text_y);
                    desc_text.size = iced::Pixels(text_size);
                    desc_text.color = if is_selected {
                        Color::from_rgb8(40, 40, 40)
                    } else {
                        Color::from_rgb8(160, 160, 160)
                    };
                    frame.fill_text(desc_text);
                    text_y += text_size * 1.3;
                }
            }
        });

        vec![geometry]
    }
}
