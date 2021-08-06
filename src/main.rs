mod wayland;

use snui::*;
use snui::widgets::*;
use snui::wayland::app;

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

    let pointer = env.get_all_seats()[0].get_pointer();
    app::quick_assign_pointer::<app::Application>(&pointer, None);

    let mempool = env.create_auto_pool();
    let surface = env.create_surface().detach();
    let layer_shell = env.require_global::<ZwlrLayerShellV1>();
    let layer_surface = layer_shell
        .get_layer_surface(&surface, None, Layer::Top, String::from("overlay"));
    layer_surface.set_exclusive_zone(-1);

    let mut applications = vec![app::Application::new(
        create_widget(7),
        surface.clone(),
        mempool.unwrap(),
    )];

    applications[0].attach_layer_surface(&layer_surface);

    let status_manager = env.require_global::<ZriverStatusManagerV1>();
    for output in &env.get_all_outputs() {
        let mut tag_list = Vec::new();
        let output_status = status_manager.get_river_output_status(&output);
        output_status.quick_assign(move |_, event, mut applications | {
            match event {
                zriver_output_status_v1::Event::FocusedTags { tags } => {
                    let applications = applications.get::<Vec<app::Application>>().unwrap();
                    for app in applications {
                        app.widget.send_action(Action::Data("occupied", &tag_list));
                        app.send_action(Action::Data("focused", &tags));
                        break;
                    }
                }
                zriver_output_status_v1::Event::ViewTags { tags } => {
                    let len = tags.len();
                    for _ in applications.get::<Vec<app::Application>>().unwrap() {
                        tag_list = (0..len).into_iter().step_by(4).map(|i| {
                            let buf: [u8; 4] = [tags[i], tags[i + 1], tags[i + 2], tags[i + 3]];
                            u32::from_le_bytes(buf)
                        }).collect();
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
pub fn create_widget(amount: u32) -> Border<Background<Rectangle, Wbox>> {
    let hl = Rectangle::square(16, YEL);

    let mut bar = Wbox::new(Orientation::Horizontal);
    bar.set_spacing(5);

    for n in 0..amount {
        let tag = 1 << n;
        let icon = Background::new(Border::new(hl, 2, BG2), Rectangle::square(0,BG1), 15);
        let action = Actionnable::new(icon, move |icon, action| {
            if action.eq("focused") {
                if let Some(focused) = action.get::<u32>() {
                    if focused == &tag || (focused / tag) % 2 != 0 {
                        icon.widget.set_color(BG2);
                        icon.widget.widget.set_color(YEL)
                    }
                }
            } else if action.eq("occupied") {
                if let Some(tags) = action.get::<Vec<u32>>() {
                    icon.widget.set_color(BG1);
                    icon.widget.widget.set_color(BG1);
                    for t in tags {
                        if t == &tag {
                            icon.widget.set_color(BG2);
                            icon.widget.widget.set_color(GRN);
                        }
                    }
                }
            }
        });
        bar.add(action).unwrap();
    }

    bar.set_color(BG0);
    boxed(bar, 5, 1, BG0, BG2)
}
