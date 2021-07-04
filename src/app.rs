use snui::snui::*;
use snui::widgets::*;
use snui::wayland::*;
use snui::widgets::{
    List,
    Button,
    Wbox,
    Rectangle
};
use wayland_client::protocol::{
    wl_surface::WlSurface,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{
        Layer,
        ZwlrLayerShellV1
    },
    zwlr_layer_surface_v1::{
        ZwlrLayerSurfaceV1,
    },
    zwlr_layer_surface_v1,
};
use wayland_client::Main;
use std::process::{Command};
use smithay_client_toolkit::shm::AutoMemPool;

pub struct App {
    pub configured: bool,
    pub focused: u32,
    pub tag_list: Vec<u32>,
    pub overlay: List,
    pub mempool: AutoMemPool,
    pub surface: Main<WlSurface>,
    pub layer_surface: Main<ZwlrLayerSurfaceV1>,
}

impl Drawable for App {
    fn set_content(&mut self, content: Content) {
        self.overlay.set_content(content);
    }
    fn get_width(&self) -> u32 {
        self.overlay.get_width()
    }
    fn get_height(&self) -> u32 {
        self.overlay.get_height()
    }
    fn draw(&self, canvas: &mut Surface, x: u32, y: u32) {
        self.overlay.draw(canvas, x, y);
    }
    fn contains(&mut self, x: u32, y: u32, event: Input) -> bool {
        self.overlay.contains(x, y, event)
    }
}

impl App {
    pub fn redraw(&mut self) {
        let mut buffer = Buffer::new(
            self.overlay.get_width() as i32,
            self.overlay.get_height() as i32 + 10,
            (4 * self.overlay.get_width()) as i32,
            &mut self.mempool,
        );
    	self.layer_surface.set_size(
        	self.overlay.get_width(),
        	self.overlay.get_height(),
    	);
    	self.overlay = create_widget(self.focused, 10, &self.tag_list);
        buffer.composite(&self.overlay.to_surface(), 0, 0);
        self.surface.damage(0, 0, self.overlay.get_width() as i32, self.overlay.get_height() as i32);
        buffer.attach(&self.surface,0,0);
    }
    pub fn commit(&mut self) {
        self.surface.commit();
    }
    pub fn new(overlay: List, surface: Main<WlSurface>, layer_surface: Main<ZwlrLayerSurfaceV1>, mempool: AutoMemPool) -> App {
    	layer_surface.set_size(10, 10);

        layer_surface.quick_assign(move |layer_surface, event, mut app| match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                println!("ping");
                let app = app.get::<App>().unwrap();
                // Configuring the surface
                layer_surface.ack_configure(serial);
                layer_surface.set_size(width, height);

                app.redraw();
            }
            zwlr_layer_surface_v1::Event::Closed => {
                println!("over");
                let app = app.get::<App>().unwrap();
                layer_surface.destroy();
                app.surface.destroy();
            }
            _ => {}
        });
        App {
            configured: false,
            focused: 0,
            tag_list: Vec::new(),
            overlay,
            surface,
            layer_surface,
            mempool
        }
    }
}

const BG0: u32 = 0xff_26_25_25;
const BG1: u32 = 0xff_33_32_32;
const BG2: u32 = 0xff_40_3e_3e;
const YEL: u32 = 0xff_c6_aa_82;
const GRN: u32 = 0xff_8D_98_7E;

pub fn create_widget(focused: u32, amount: u32, occupied: &Vec<u32>) -> List {
    // Creating the widget
    let bg = Rectangle::square(50, Content::Pixel(BG0));
    let bg1 = Rectangle::square(50, Content::Pixel(BG1));
    let hl = Rectangle::square(10, Content::Pixel(YEL));
    let hl2 = Rectangle::square(10, Content::Pixel(GRN));
    let buttons: Vec<Button> = (0..10).map(|n| {
        if n == focused {
            let mut focused_icon = Wbox::new(bg1);
            focused_icon.add(hl).unwrap();
            Button::new(focused_icon, |input| {
                match input {
                    Input::MouseClick{ time, button, pressed } => {
                        println!("focused")
                    }
                    _ => {}
                }
            })
        } else {
            let mut occupied_icon = Wbox::new(bg1);
            if n == n {
                occupied_icon.add(hl2).unwrap();
            } else {
                occupied_icon.add(bg1).unwrap();
            }
            Button::new(occupied_icon, |input| {
                match input {
                    Input::MouseClick{ time, button, pressed } => {
                        println!("unfocused")
                    }
                    _ => {}
                }
            })
        }
    }).collect();

	// Addind the created buttons to the bar
    let mut bar = List::new(Orientation::Horizontal, None);
	bar.set_content(Content::Pixel(BG2));
	bar.set_margin(10);
	for b in buttons {
    	bar.add(b);
	}
    bar
}

fn run_command(value: String) {
    let mut string = value.split_whitespace();
    let mut command = Command::new(string.next().unwrap());
    command.args(string.collect::<Vec<&str>>());
    command.spawn().expect("Error");
}
