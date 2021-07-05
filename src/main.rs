mod wayland;
mod app;
mod environment;
use snui::wayland::input;
use snui::widgets::{
    List,
    Button,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;
use snui::wayland::buffer::Buffer;
use smithay_client_toolkit::shm::{AutoMemPool, Format};
use environment::Environment;
use wayland_client::{Display, EventQueue, Main, Attached};
use std::thread;
use std::time::Duration;

use crate::wayland::river_status_unstable_v1::{
    zriver_output_status_v1,
    zriver_seat_status_v1,
    zriver_status_manager_v1::ZriverStatusManagerV1,
};

fn main() {
    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let mut environment = Environment::new(&display, &mut event_queue);

    // Mempool
    let attached = Attached::from(environment.shm.clone().expect("No shared memory pool"));
    let mut mempool = AutoMemPool::new(attached).unwrap();

    // Getting the pointer
    let pointer = environment.seats[0].get_pointer();

    // Creating widget
    let mut widget = app::create_widget(0, 7, &Vec::new());

    input::assign_pointer::<app::App>(&pointer);

	let mut focused_tag = 0;
	let surface = environment.get_surface();
    let layer_surface = environment
        .layer_shell
        .as_ref()
        .expect("Compositor doesn't implement the LayerShell protocol")
        .get_layer_surface(&surface, None, Layer::Top, String::from("overlay"));

	for output in &environment.outputs {
        let output_status = environment
            .status_manager
            .as_ref()
            .expect("Compositor doesn't implement river_status_unstable_v1")
            .get_river_output_status(&output.wl_output);

        output_status.quick_assign(move |_, event, mut app| match event {
            zriver_output_status_v1::Event::FocusedTags { tags } => {
                let mut app = app.get::<app::App>().unwrap();
                app.focused = tags;
                if app.configured {
                    app.redraw();
                }
            }
            zriver_output_status_v1::Event::ViewTags { tags } => {
                let mut app = app.get::<app::App>().unwrap();
                let len = tags.len();
                for i in (0..len).into_iter().step_by(4) {
                    let buf: [u8; 4] = [tags[i], tags[i + 1], tags[i + 2], tags[i + 3]];
                    app.tag_list.push(u32::from_le_bytes(buf));
                }
            }
        });
	}

    let mut app = app::App::new(widget, surface, layer_surface, mempool);

    loop {
        event_queue
            .dispatch(&mut app, |event, object, _| {
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


