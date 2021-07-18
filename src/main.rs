mod environment;
mod overlay;
mod wayland;
use snui::snui::*;
use snui::wayland::*;
use std::io::{BufWriter, Write};
use environment::Environment;
use smithay_client_toolkit::shm::AutoMemPool;
use snui::wayland::app;
use snui::wayland::app::LayerSurface;
use wayland_client::{Attached, Display};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;

const TRANSPARENT: u32 = 0x00_00_00_00;

use crate::wayland::river_status_unstable_v1::zriver_output_status_v1;

fn main() {
    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let environment = Environment::new(&display, &mut event_queue);

    // Mempool
    let attached = Attached::from(environment.shm.clone().expect("No shared memory pool"));
    let mempool = AutoMemPool::new(attached).unwrap();

    // Getting the pointer
    let pointer = environment.seats[0].get_pointer();

    // Creating widget
    let widget = overlay::create_widget(1, 4, &vec![]);

    let surface = environment.get_surface();
    let layer_surface = environment
        .layer_shell
        .as_ref()
        .expect("Compositor doesn't implement the LayerShell protocol")
        .get_layer_surface(&surface, None, Layer::Top, String::from("overlay"));

    app::assign_pointer(&pointer, |damage, app: &mut overlay::App| match damage{
        Damage::All { surface } => {
            println!("reeeeee");
            app.composite(&surface, 0, 0);
        }
        Damage::Area { surface, x, y } => {
            println!("reeeeee");
            let mut buffer = Buffer::new(
                app.overlay.get_width() as i32,
                app.overlay.get_height() as i32 + 10,
                (4 * app.overlay.get_width()) as i32,
                &mut app.mempool,
            );
            app.surface.damage(
                x as i32,
                y as i32,
                surface.get_width() as i32,
                surface.get_height() as i32,
            );
            buffer.composite(&surface, x, y);
            buffer.attach(&app.surface,0, 0);
        }
        Damage::Destroy => {
            println!("reeeeee");
            let size = app.size();
            let mut buf = BufWriter::new(app.get_mut_buf());
            for _ in 0..size {
                buf.write_all(&TRANSPARENT.to_ne_bytes()).unwrap();
            }
            buf.flush().unwrap();
        }
        _ => {}
    });
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
