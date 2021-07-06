use smithay_client_toolkit::shm::AutoMemPool;
use snui::snui::*;
use snui::wayland::*;
use snui::widgets::*;
use snui::widgets::{Button, List, Rectangle, Wbox};
use std::process::Command;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::Main;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_surface_v1, zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
};

const BG0: u32 = 0xff_26_25_25;
const BG1: u32 = 0xff_33_32_32;
const BG2: u32 = 0xff_40_3e_3e;
const YEL: u32 = 0xff_c6_aa_82;
const GRN: u32 = 0xff_98_96_7E;

pub struct App {
    pub hidden: bool,
    pub configured: bool,
    pub focused: u32,
    pub tag_list: Vec<u32>,
    pub overlay: List,
    pub buffer: Surface,
    pub mempool: AutoMemPool,
    pub surface: Main<WlSurface>,
    pub layer_surface: Main<ZwlrLayerSurfaceV1>,
}

impl Geometry for App {
    fn get_width(&self) -> u32 {
        self.overlay.get_width()
    }
    fn get_height(&self) -> u32 {
        self.overlay.get_height()
    }
    fn get_location(&self) -> (u32, u32) {
        (0, 0)
    }
    fn set_location(&mut self, _x: u32, _y: u32) { }
    fn contains(&mut self, x: u32, y: u32, event: Input) -> Damage {
        self.overlay.contains(x, y, event)
    }
}

impl Canvas for App {
    fn paint(&self) {}
    fn damage(&mut self, event: Damage) {
        match event {
            Damage::Area { surface, x, y } => {
                self.buffer.composite(&surface, x, y);
                let mut buffer = Buffer::new(
                    self.overlay.get_width() as i32,
                    self.overlay.get_height() as i32 + 10,
                    (4 * self.overlay.get_width()) as i32,
                    &mut self.mempool,
                );
                buffer.composite(&self.buffer, 0, 0);
                buffer.attach(&self.surface, 0, 0);
                self.surface.damage(
                    x as i32,
                    y as i32,
                    surface.get_width() as i32,
                    surface.get_height() as i32,
                );
                self.surface.commit();
            }
            Damage::Own => self.redraw(),
            _ => {}
        }
    }
    fn get(&self, _x: u32, _y: u32) -> Content { Content::Empty }
    fn set(&mut self, _x: u32, _y: u32, _content: Content) { }
    fn composite(&mut self, surface: &(impl Canvas + Geometry), x: u32, y: u32) {
        let mut buffer = Buffer::new(
            self.overlay.get_width() as i32,
            self.overlay.get_height() as i32 + 10,
            (4 * self.overlay.get_width()) as i32,
            &mut self.mempool,
        );
        buffer.composite(surface, x, y);
        buffer.attach(&self.surface, 0, 0);
        self.surface.damage(
            0,
            0,
            surface.get_width() as i32,
            surface.get_height() as i32,
        );
        self.surface.commit();
    }
}

impl App {
    pub fn redraw(&mut self) {
        self.hidden = false;
        let mut buffer = Buffer::new(
            self.overlay.get_width() as i32,
            self.overlay.get_height() as i32 + 10,
            (4 * self.overlay.get_width()) as i32,
            &mut self.mempool,
        );
        self.layer_surface
            .set_size(self.overlay.get_width(), self.overlay.get_height());
        self.overlay = create_widget(self.focused, 7, &self.tag_list);
        self.buffer = to_surface(&self.overlay);
        buffer.composite(&self.buffer, 0, 0);
        buffer.attach(&self.surface, 0, 0);
        self.surface.damage(
            0,
            0,
            self.overlay.get_width() as i32,
            self.overlay.get_height() as i32,
        );
        self.surface.commit();
    }
    pub fn commit(&mut self) {
        self.surface.commit();
    }
    pub fn new(
        overlay: List,
        surface: Main<WlSurface>,
        layer_surface: Main<ZwlrLayerSurfaceV1>,
        mempool: AutoMemPool,
    ) -> App {
        layer_surface.set_size(overlay.get_width(), overlay.get_height());
        surface.commit();

        layer_surface.quick_assign(move |layer_surface, event, mut app| match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                let app = app.get::<App>().unwrap();
                layer_surface.ack_configure(serial);
                layer_surface.set_size(width, height);

                app.configured = true;
                if !app.hidden {
                    app.redraw();
                }
                app.commit();
            }
            zwlr_layer_surface_v1::Event::Closed => {
                let app = app.get::<App>().unwrap();
                layer_surface.destroy();
                app.surface.destroy();
            }
            _ => {}
        });
        App {
            configured: false,
            hidden: false,
            focused: 0,
            buffer: Surface::empty(1, 1),
            tag_list: Vec::new(),
            overlay,
            surface,
            layer_surface,
            mempool,
        }
    }
}

pub fn create_widget(mut focused: u32, amount: u32, occupied: &Vec<u32>) -> List {
    let bg = Rectangle::square(60, Content::Pixel(BG0));
    let sl = Rectangle::square(24, Content::Pixel(BG2));
    let hl = Rectangle::square(20, Content::Pixel(YEL));
    let hl2 = Rectangle::square(20, Content::Pixel(GRN));

    let mut bar = List::new(Orientation::Horizontal, None);
    bar.set_content(Content::Pixel(BG1));
    bar.set_margin(10);

    let mut current;
    for n in 0..amount {
        if {
            current = 1 << n;
            current == focused || (focused / current) % 2 != 0
        } {
            focused -= current;
            let mut focused_icon = Wbox::new(bg);
            focused_icon.center(sl).unwrap();
            focused_icon.center(hl).unwrap();
            bar.add(Button::new(focused_icon, |child, input| match input {
                Input::MouseClick {
                    time: _,
                    button: _,
                    pressed,
                } => {
                    let (x, y) = child.get_location();
                    if pressed {
                        let size = child.get_width();
                        let widget = Rectangle::square(size, Content::Pixel(GRN));
                        Damage::Area{
                            surface: to_surface(&widget),
                            x,
                            y
                        }
                    } else {
                        Damage::Area{
                            surface: to_surface(child),
                            x,
                            y
                        }
                    }
                }
                _ => Damage::None
            }))
            .unwrap();
        } else {
            let mut occupied_icon = Wbox::new(bg);
            if {
                let mut valid = false;
                for tag in occupied {
                    if 1 << n == *tag {
                        valid = true;
                    }
                }
                valid
            } {
                occupied_icon.center(sl).unwrap();
                occupied_icon.center(hl2).unwrap();
            } else {
                occupied_icon.center(bg).unwrap();
            }
            bar.add(Button::new(occupied_icon, |_child, _input| {Damage::None})).unwrap();
        }
    }
    bar
}

fn _run_command(value: String) {
    let mut string = value.split_whitespace();
    let mut command = Command::new(string.next().unwrap());
    command.args(string.collect::<Vec<&str>>());
    command.spawn().expect("Error");
}
