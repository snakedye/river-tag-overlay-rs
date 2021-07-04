mod wayland;
mod app;
mod environment;
use snui::snui::*;
use snui::widgets::{
    List,
    Button,
};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;
use snui::wayland::buffer::Buffer;
use smithay_client_toolkit::shm::{AutoMemPool, Format};
use environment::Environment;
use wayland_client::{Display, EventQueue, Main, Attached};

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
    let mut widget = app::create_widget(0, 9, &Vec::new());

	let mut focused_tag = 0;
	let surface = environment.get_surface();
    let layer_surface = environment
        .layer_shell
        .as_ref()
        .expect("Compositor doesn't implement the LayerShell protocol")
        .get_layer_surface(&surface, None, Layer::Top, String::from("test"));

	for output in &environment.outputs {
    	let surface_handle = surface.clone();
        let output_status = environment
            .status_manager
            .as_ref()
            .expect("Compositor doesn't implement river_status_unstable_v1")
            .get_river_output_status(&output.wl_output);

        output_status.quick_assign(move |_, event, mut app| match event {
            zriver_output_status_v1::Event::FocusedTags { tags } => {
                let mut app = app.get::<app::App>().unwrap();
                app.focused = tag(tags);
                if app.configured {
                    println!("pong");
                    app.redraw();
                    app.commit();
                } else {
                    app.configured = true;
                }
            }
            zriver_output_status_v1::Event::ViewTags { tags } => {
                let mut app = app.get::<app::App>().unwrap();
                let len = tags.len();
                for i in (0..len).into_iter().step_by(4) {
                    let buf: [u8; 4] = [tags[i], tags[i + 1], tags[i + 2], tags[i + 3]];
                    app.tag_list.push(tag(u32::from_le_bytes(buf)));
                }
            }
        });
	}

    let mut app = app::App::new(widget, surface, layer_surface, mempool);
    app.commit();

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


fn tag(tagmask: u32) -> u32 {
    let mut int = 0;
    let mut current: u32;
    while {
        current = 1 << int;
        current < tagmask
    } {
        int += 1;
        if current != tagmask && (tagmask / current) % 2 != 0 {
            int = tag(tagmask - current);
            break;
        }
    }
    int
}
