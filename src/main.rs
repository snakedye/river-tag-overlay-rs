mod wayland;

use snui::*;
use snui::wayland::app;
use snui::wayland::app::LayerSurface;
use snui::widgets::*;

const BG0: u32 = 0xff_26_25_25;
const BG1: u32 = 0xff_33_32_32;
const BG2: u32 = 0xff_40_3e_3e;
const YEL: u32 = 0xff_c6_aa_82;
const GRN: u32 = 0xff_98_96_7E;

use smithay_client_toolkit::{
    output::OutputHandler,
    seat::SeatHandler,
    shm::ShmHandler,
    environment,
    environment::{
        Environment,
        SimpleGlobal,
    }
};
use wayland_client::protocol::{
    wl_compositor::WlCompositor,
    wl_output::WlOutput,
    wl_seat::WlSeat,
    wl_shm::WlShm,
};
use wayland_client::{Display, Proxy};
use crate::wayland::river_status_unstable_v1::zriver_status_manager_v1::ZriverStatusManagerV1;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;

use crate::wayland::river_status_unstable_v1::zriver_output_status_v1;

pub struct Env {
    compositor: SimpleGlobal<WlCompositor>,
    layer_shell: SimpleGlobal<ZwlrLayerShellV1>,
    status_manager: SimpleGlobal<ZriverStatusManagerV1>,
    shm: ShmHandler,
    outputs: OutputHandler,
    seats: SeatHandler,
}

impl Env {
    fn new() -> Env {
        Env {
            compositor: SimpleGlobal::new(),
            layer_shell: SimpleGlobal::new(),
            status_manager: SimpleGlobal::new(),
            shm: ShmHandler::new(),
            outputs: OutputHandler::new(),
            seats: SeatHandler::new()
        }
    }
}

environment!(Env,
    singles = [
        ZwlrLayerShellV1 => layer_shell,
        ZriverStatusManagerV1 => status_manager,
       	WlCompositor => compositor,
       	WlShm => shm,
    ],
    multis=[
        WlOutput => outputs,
        WlSeat => seats,
    ]
);

fn main() {
    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let wl_display = Proxy::clone(&display).attach(event_queue.token());

    let env = Environment::new(&wl_display, &mut event_queue, Env::new()).unwrap();
    let pointer = env.get_all_seats()[0].get_pointer();
    app::quick_assign_pointer::<app::Application>(&pointer, None);

    let mut applications: Vec<app::Application> = Vec::new();
    let pointer = env.get_all_seats()[0].get_pointer();
    app::quick_assign_pointer::<app::Application>(&pointer, None);

    let mempool = env.create_auto_pool();
    let surface = env.create_surface().detach();
    let layer_shell = env.require_global::<ZwlrLayerShellV1>();
    let layer_surface = layer_shell
        .get_layer_surface(&surface, None, Layer::Top, String::from("overlay"));
    layer_surface.set_exclusive_zone(-1);

    app::assign_layer_surface::<app::Application>(&surface, &layer_surface);
    applications.push(app::Application::new(
        create_widget(0, 7, &vec![]),
        surface,
        &layer_surface,
        mempool.unwrap(),
    ));

    let status_manager = env.require_global::<ZriverStatusManagerV1>();
    for output in &env.get_all_outputs() {
        let mut tag_list = Vec::new();
        let output_status = status_manager.get_river_output_status(&output);
        output_status.quick_assign(move |_, event, mut applications | {
            match event {
                zriver_output_status_v1::Event::FocusedTags { tags } => {
                    let applications = applications.get::<Vec<app::Application>>().unwrap();
                    for widget in applications {
                        widget.widget = Box::new(create_widget(tags, 7, &tag_list));
                        widget.show();
                        break;
                    }
                }
                zriver_output_status_v1::Event::ViewTags { tags } => {
                    let len = tags.len();
                    for _ in applications.get::<Vec<app::Application>>().unwrap() {
                        tag_list = Vec::new();
                        for i in (0..len).into_iter().step_by(4) {
                            let buf: [u8; 4] = [tags[i], tags[i + 1], tags[i + 2], tags[i + 3]];
                            tag_list.push(u32::from_le_bytes(buf));
                        }
                        break;
                    }
                }
            }
        });
    }

    loop {
        event_queue
            .dispatch(&mut applications, |event, object, _| {
                panic!(
                    "[callop] Encountered an orphan event: {}@{}: {}",
                    event.interface,
                    object.as_ref().id(),
                    event.name
                );
            })
            .unwrap();
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
            focused_icon
                .center(border(hl, 2, Content::Pixel(BG2)))
                .unwrap();
            bar.add(Button::new(
                focused_icon,
                move |child, input| match input {
                    Input::MouseClick {
                        time: _,
                        button: _,
                        pressed,
                    } => {
                        if pressed {
                            child.set_content(Content::Pixel(GRN));
                        } else {
                            child.set_content(Content::Pixel(BG0));
                        }
                        true
                    }
                    _ => false,
                },
            ))
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
                occupied_icon
                    .center(border(hl2, 2, Content::Pixel(BG2)))
                    .unwrap();
            } else {
                occupied_icon.center(bg).unwrap();
            }
            bar.add(occupied_icon).unwrap();
        }
    }
    bar
}
