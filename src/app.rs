use snui::snui::*;
use snui::wayland::*;
use snui::wayland::buffer::*;
use snui::widgets::*;
use std::process::Command;
use smithay_client_toolkit::shm::MemPool;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::Main;
use snui::wayland::utils::LayerSurface;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_surface_v1,
    zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
};

const BG0: u32 = 0xff_26_25_25;
const BG1: u32 = 0xff_33_32_32;
const BG2: u32 = 0xff_40_3e_3e;
const YEL: u32 = 0xff_c6_aa_82;
const GRN: u32 = 0xff_98_96_7E;

pub struct App {
    pub configured: bool,
    pub focused: u32,
    pub tag_list: Vec<u32>,
    pub overlay: ListBox,
    pub buffer: Buffer,
    pub mempool: Pool,
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

impl App {
    pub fn new(
        overlay: ListBox,
        surface: Main<WlSurface>,
        layer_surface: Main<ZwlrLayerSurfaceV1>,
        mempool: MemPool,
    ) -> App {
        let width = overlay.get_width();
        let height = overlay.get_height();
        let mut mempool = Pool::new(width as i32, height as i32, 4, mempool);
        let buffer = mempool.new_buffer(width as i32, height as i32);
        layer_surface.set_size(overlay.get_width(), overlay.get_height());
        layer_surface.quick_assign(move |layer_surface, event, mut shell| {
            match event {
                zwlr_layer_surface_v1::Event::Configure {
                    serial,
                    width,
                    height,
                } => {
                    let shell = shell.get::<App>().unwrap();
                    shell.mempool.resize(width as i32, height as i32);
                    layer_surface.ack_configure(serial);
                    layer_surface.set_size(width, height);
                    println!("configure");

                    // The client should use commit to notify itself
                    // that it has been configured
                    // The client is also responsible for damage
                    shell.redraw();
                }
                zwlr_layer_surface_v1::Event::Closed => {
                    let shell = shell.get::<App>().unwrap();
                    layer_surface.destroy();
                    shell.surface.destroy();
                }
                _ => {}
            }
        });
        surface.commit();
        buffer.attach(&surface);
        App {
            configured: false,
            focused: 0,
            buffer,
            tag_list: Vec::new(),
            overlay,
            surface,
            layer_surface,
            mempool,
        }
    }
    pub fn redraw(&mut self) {
        self.overlay = create_widget(self.focused, 7, &self.tag_list);
        let buffer = self.mempool.new_buffer(
            self.overlay.get_width() as i32,
            self.overlay.get_height() as i32,
        );
        self.mempool.composite(0, 0, &buffer, to_surface(&self.overlay).get_buf());
        buffer.attach(&self.surface);
        self.configured = true;
        self.surface.damage(
            0,
            0,
            self.overlay.get_width() as i32,
            self.overlay.get_height() as i32,
        );
        self.surface.commit();
    }
    fn damage(&mut self, event: Damage) {
        match event {
            Damage::Area { mut surface, x, y } => {
                self.mempool.composite(x as i32, y as i32, &self.buffer, surface.get_buf());
                self.surface.commit();
            }
            Damage::Own => self.redraw(),
            _ => {}
        }
    }
}

pub fn create_widget(mut focused: u32, amount: u32, occupied: &Vec<u32>) -> ListBox {
    let bg = Rectangle::square(60, Content::Pixel(BG0));
    let sl = Rectangle::square(24, Content::Pixel(BG2));
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
            let mut focused_icon = Wbox::new(bg);
            focused_icon.center(sl).unwrap();
            focused_icon.center(Button::new(hl, |child, x, y, input| match input {
                Input::MouseClick {
                    time: _,
                    button: _,
                    pressed,
                } => {
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
                _ => {
                    Damage::None
                }
            })).unwrap();
            bar.add(focused_icon).unwrap();
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
            bar.add(occupied_icon).unwrap();
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
