use smithay_client_toolkit::shm::AutoMemPool;
use snui::snui::*;
use snui::wayland::*;
use snui::widgets::*;
use snui::widgets::{Button, ListBox, Rectangle, Node};
use std::process::Command;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::Main;
use snui::wayland::app::LayerSurface;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
};

const BG0: u32 = 0xff_26_25_25;
const BG1: u32 = 0xff_33_32_32;
const BG2: u32 = 0xff_40_3e_3e;
const YEL: u32 = 0xff_c6_aa_82;
const GRN: u32 = 0xff_98_96_7E;

fn _run_command(value: String) {
    let mut string = value.split_whitespace();
    let mut command = Command::new(string.next().unwrap());
    command.args(string.collect::<Vec<&str>>());
    command.spawn().expect("Error");
}

pub struct App {
    pub configured: bool,
    pub focused: u32,
    pub tag_list: Vec<u32>,
    pub overlay: ListBox,
    pub pixmap: Surface,
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
    fn contains(&mut self, widget_x: u32, widget_y: u32, x: u32, y: u32, event: Input) -> Damage {
        self.overlay.contains(widget_x, widget_y, x, y, event)
    }
}

impl LayerSurface for App {
    fn get_surface(&self) -> &Main<WlSurface> {
        &self.surface
    }
    fn resize(&mut self, width: u32, height: u32) {
        self.mempool.resize((width*height) as usize).unwrap();
    }
    fn display(&mut self) {
        self.configured = true;
        let mut buffer = Buffer::new(
            self.overlay.get_width() as i32,
            self.overlay.get_height() as i32 + 10,
            (4 * self.overlay.get_width()) as i32,
            &mut self.mempool,
        );
        self.layer_surface
            .set_size(self.overlay.get_width(), self.overlay.get_height());
        self.overlay = create_widget(self.focused, 7, &self.tag_list);
        self.pixmap = to_surface(&self.overlay);
        buffer.composite(&self.pixmap, 0, 0);
        buffer.attach(&self.surface, 0, 0);
        self.surface.damage(
            0,
            0,
            self.overlay.get_width() as i32,
            self.overlay.get_height() as i32,
        );
        self.surface.commit();
    }
}

impl Canvas for App {
    fn damage(&mut self, event: Damage) {
        match event {
            Damage::Area { surface, x, y } => {
                self.composite(&surface, x, y);
                let mut buffer = Buffer::new(
                    self.overlay.get_width() as i32,
                    self.overlay.get_height() as i32 + 10,
                    (4 * self.overlay.get_width()) as i32,
                    &mut self.mempool,
                );
                buffer.composite(&self.pixmap, 0, 0);
                buffer.attach(&self.surface, 0, 0);
                self.surface.damage(
                    x as i32,
                    y as i32,
                    surface.get_width() as i32,
                    surface.get_height() as i32,
                );
                self.surface.commit();
            }
            Damage::Own => self.display(),
            _ => {}
        }
    }
    fn get(&self, _x: u32, _y: u32) -> Content { Content::Empty }
    fn set(&mut self, _x: u32, _y: u32, _content: Content) { }
    fn composite(&mut self, surface: &(impl Canvas + Geometry), x: u32, y: u32) {
        self.pixmap.composite(surface, x, y);
    }
}

impl App {
    pub fn new(
        overlay: ListBox,
        surface: Main<WlSurface>,
        layer_surface: Main<ZwlrLayerSurfaceV1>,
        mempool: AutoMemPool,
    ) -> App {
        layer_surface.set_size(overlay.get_width(), overlay.get_height());
        surface.commit();
        App {
            configured: false,
            focused: 0,
            pixmap: Surface::empty(0, 0),
            tag_list: Vec::new(),
            overlay,
            surface,
            layer_surface,
            mempool,
        }
    }
}

pub fn create_widget(mut focused: u32, amount: u32, occupied: &Vec<u32>) -> ListBox {
    let bg = Rectangle::square(60, Content::Pixel(BG0));
    let hl = Rectangle::square(20, Content::Pixel(YEL));
    let hl2 = Rectangle::square(20, Content::Pixel(GRN));

    let mut bar = ListBox::new(Orientation::Horizontal);
    bar.set_content(Content::Pixel(BG1));
    bar.set_margin(10);

    let mut current;
    for n in 0..amount {
        if {
            current = 1 << n;
            current == focused || (focused / current) % 2 != 0
        } {
            focused -= current;
            let mut focused_icon = Node::new(bg);
            focused_icon.center(border(hl, 3, Content::Pixel(BG2))).unwrap();
            bar.add(Button::new(focused_icon, move |child, x, y, input| match input {
                Input::MouseClick {
                    time: _,
                    button: _,
                    pressed,
                } => {
                    if pressed {
                        child.set_content(Content::Pixel(GRN));
                        Damage::Area{
                            surface: to_surface(child),
                            x,
                            y
                        }
                    } else {
                        child.set_content(Content::Pixel(BG0));
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
            let mut occupied_icon = Node::new(bg);
            if {
                let mut valid = false;
                for tag in occupied {
                    if 1 << n == *tag {
                        valid = true;
                    }
                }
                valid
            } {
                occupied_icon.center(border(hl2, 2, Content::Pixel(BG2))).unwrap();
            } else {
                occupied_icon.center(bg).unwrap();
            }
            bar.add(occupied_icon).unwrap();
        }
    }
    bar
}
