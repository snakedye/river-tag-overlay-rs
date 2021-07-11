mod overlay;
mod wayland;
mod environment;
use snui::wayland::app;
use environment::Environment;
use snui::wayland::app::LayerSurface;
use wayland_client::{Attached, Display};
use smithay_client_toolkit::shm::AutoMemPool;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;

use crate::wayland::river_status_unstable_v1::zriver_output_status_v1;

fn main() {
    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let environment = Environment::new(&display, &mut event_queue);

    // Mempool
    let attached = Attached::from(environment.shm.clone().expect("No shared memory pool"));
    let mempool = MemPool::new(attached, |_| {}).unwrap();

    // Getting the pointer
    let pointer = environment.seats[0].get_pointer();

    // Creating widget
    let widget = overlay::create_widget(0, 7, &Vec::new());

    let surface = environment.get_surface();
    let layer_surface = environment
        .layer_shell
        .as_ref()
        .expect("Compositor doesn't implement the LayerShell protocol")
        .get_layer_surface(&surface, None, Layer::Top, String::from("overlay"));

    app::assign_pointer::<overlay::App>(&pointer);
    app::assign_layer_surface::<overlay::App>(&layer_surface);

    for output in &environment.outputs {
        let output_status = environment
            .status_manager
            .as_ref()
            .expect("Compositor doesn't implement river_status_unstable_v1")
            .get_river_output_status(&output.wl_output);

        output_status.quick_assign(move |_, event, mut overlay| match event {
            zriver_output_status_v1::Event::FocusedTags { tags } => {
                let mut overlay = overlay.get::<overlay::App>().unwrap();
                overlay.focused = tags;
                if overlay.configured {
                    overlay.display();
                }
            }
            zriver_output_status_v1::Event::ViewTags { tags } => {
                let overlay = overlay.get::<overlay::App>().unwrap();
                let len = tags.len();
                for i in (0..len).into_iter().step_by(4) {
                    let buf: [u8; 4] = [tags[i], tags[i + 1], tags[i + 2], tags[i + 3]];
                    overlay.tag_list.push(u32::from_le_bytes(buf));
                }
            }
        });
    }

    let mut overlay = overlay::App::new(widget, surface, layer_surface, mempool);

    loop {
        event_queue
            .dispatch(&mut overlay, |event, object, _| {
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
