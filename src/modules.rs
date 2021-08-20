use snui::*;
use snui::widgets::*;

const BG1: u32 = 0xff_33_32_32;
const BG2: u32 = 0xff_51_50_50;
const YEL: u32 = 0xff_c6_aa_82;
const RED: u32 = 0xff_b7_66_66;
const GRN: u32 = 0xff_98_96_7E;

pub struct TagButton {
    tag: u32,
    button: Button<Rectangle>,
}

impl Geometry for TagButton {
    fn get_width(&self) -> u32 {
        self.button.get_width()
    }
    fn get_height(&self) -> u32 {
        self.button.get_height()
    }
    fn resize(&mut self, _width: u32, _height: u32) -> Result<(), Error> {
        Ok(())
    }
    fn contains<'d>(
        &'d mut self,
        widget_x: u32,
        widget_y: u32,
        x: u32,
        y: u32,
        event: Event,
    ) -> Damage {
        self.button.contains(widget_x, widget_y, x, y, event)
    }
}

impl Drawable for TagButton {
    fn set_color(&mut self, color: u32) {
        self.button.set_color(color);
    }
    fn draw(&self, canvas: &mut [u8], width: u32, x: u32, y: u32) {
        self.button.draw(canvas, width, x, y);
    }
}

impl Widget for TagButton {
    fn send_command<'s>(
        &'s mut self,
        command: Command,
        _damage_queue: &mut Vec<Damage<'s>>,
        _x: u32,
        _y: u32,
    ) {
        if command.eq("focused") {
            if let Some(focused) = command.get::<u32>() {
                if focused == &self.tag || (focused / self.tag) % 2 != 0 {
                    self.button.set_color(YEL);
                }
            }
        } else if command.eq("urgent") {
            if let Some(urgent) = command.get::<u32>() {
                if urgent == &self.tag || (urgent / self.tag) % 2 != 0 {
                    self.button.set_color(RED);
                }
            }
        } else if command.eq("occupied") {
            if let Some(tags) = command.get::<Vec<u32>>() {
                for t in tags {
                    self.button.set_color(BG1);
                    if t == &self.tag {
                        self.button.set_color(BG2);
                        break;
                    }
                }
            }
        }
    }
}

impl TagButton {
    pub fn new(tag: u32, icon_size: u32) -> Self {
        let action = widgets::button::Action::Command(format!("riverctl set-focused-tags {}", tag));
        let icon = Rectangle::square(icon_size, BG1);
        Self {
            tag,
            button: Button::new(icon, action)
        }
    }
}
