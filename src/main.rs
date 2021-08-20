mod wayland;
mod modules;

use snui::*;
use std::time;
use std::thread;
use snui::widgets::*;
use snui::wayland::Buffer;
use smithay_client_toolkit::shm::AutoMemPool;
use wayland_client::{Display, Attached, Proxy, Main};
use crate::wayland::river_status_unstable_v1::zriver_output_status_v1;
use crate::wayland::river_status_unstable_v1::zriver_status_manager_v1::ZriverStatusManagerV1;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_surface_v1;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

use wayland_client::protocol::{
    wl_surface::WlSurface,
    wl_compositor::WlCompositor,
    wl_output::WlOutput,
    wl_shm::WlShm,
};

use smithay_client_toolkit::{
    output::OutputHandler,
    shm::ShmHandler,
    environment,
    environment::{
        Environment,
        SimpleGlobal,
    }
};

pub struct Env {
    status_manager: SimpleGlobal<ZriverStatusManagerV1>,
    compositor: SimpleGlobal<WlCompositor>,
    layer_shell: SimpleGlobal<ZwlrLayerShellV1>,
    outputs: OutputHandler,
    shm: ShmHandler,
}

impl Env {
    fn new() -> Env {
        Env {
            status_manager: SimpleGlobal::new(),
            compositor: SimpleGlobal::new(),
            layer_shell: SimpleGlobal::new(),
            outputs: OutputHandler::new(),
            shm: ShmHandler::new(),
        }
    }
}

environment!(Env,
    singles = [
    	ZriverStatusManagerV1 => status_manager,
        ZwlrLayerShellV1 => layer_shell,
       	WlCompositor => compositor,
       	WlShm => shm,
    ],
    multis=[
        WlOutput => outputs,
    ]
);

const BG0: u32 = 0xff_26_25_25;
const BG1: u32 = 0xff_33_32_32;

struct App<W: Widget> {
    widget: W,
    mempool: AutoMemPool,
    compositor: Attached<WlCompositor>,
}

impl<W: Widget> App<W> {
    fn new(widget: W, mempool: AutoMemPool, compositor: Attached<WlCompositor>) -> Self {
        Self {
            widget,
            mempool,
            compositor
        }
    }
}

fn main() {
    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let wl_display = Proxy::clone(&display).attach(event_queue.token());
    let env = Environment::new(&wl_display, &mut event_queue, Env::new()).unwrap();

	let widget = create_widget(9, 40);
    let mut mempool = env.create_auto_pool().unwrap();
    let compositor = env.require_global::<WlCompositor>();
    let status_manager = env.require_global::<ZriverStatusManagerV1>();
    let draw = mempool.resize((widget.get_width() * widget.get_height() * 4) as usize).is_ok();

    let mut app = App::new(widget, mempool, compositor);

	if draw {
        for output in env.get_all_outputs() {
            let output_status =
                status_manager.get_river_output_status(&output);
            let mut viewstag: Vec<u32> = Vec::new();
            let display_handle = display.clone();
            let mut surface_queue: Vec<Main<WlSurface>> = Vec::new();
            let layer_shell = env.require_global::<ZwlrLayerShellV1>();
            output_status.quick_assign(move |_, event, mut app| {
                if let Some(app) = app.get::<App<Border<Background<WidgetLayout>>>>() {
                    match event {
                        zriver_output_status_v1::Event::FocusedTags {
                            tags,
                        } => {
                            if let Some(surface) = surface_queue.pop() {
                                surface.destroy();
                            }
                            app.widget.send_command(Command::Data("occupied", &viewstag), &mut Vec::new(), 0, 0);
                        	app.widget.send_command(Command::Data("focused", &tags), &mut Vec::new(), 0, 0);
                            let width = app.widget.get_width();
                            let height = app.widget.get_height();
                            let surface = app.compositor.create_surface();
                            let layer_surface = layer_shell
                                .get_layer_surface(&surface, None, Layer::Overlay, "overlay".to_owned());
                            surface.quick_assign(|_, _, _| {});
                            layer_surface.set_size(width, height);
                            surface.commit();

                            let mut buffer = Buffer::new(
                                width as i32,
                                height as i32,
                                width as i32 * 4,
                                &mut app.mempool
                            );
                            app.widget.draw(buffer.get_mut_buf(), width, 0, 0);

                            let surface_handle = surface.detach();
    						let display_handle = display_handle.clone();
                            layer_surface.quick_assign(move |layer_surface, event, _| match event {
                                zwlr_layer_surface_v1::Event::Configure{serial, width:_, height:_} => {
            						let display_handle = display_handle.clone();
                                    layer_surface.ack_configure(serial);
                                    surface_handle.damage( 0, 0, 1 << 30, 1 << 30 );
                                    surface_handle.commit();
                                    let layer_surface = layer_surface.detach();
                                    let surface_handle = surface_handle.clone();
                                    thread::spawn(move || {
                                        let mut event_queue = display_handle.create_event_queue();
                                        thread::sleep(time::Duration::from_millis(600));
                                        layer_surface.destroy();
                                        surface_handle.destroy();
                                        event_queue
                                            .sync_roundtrip(&mut (), |_, _, _| unreachable!()).unwrap();
                                    });
                                }
                                _ => {
                                    layer_surface.destroy();
                                }
                            });
                            buffer.attach(&surface, 0, 0);
                            surface_queue.push(surface);
                        }
                        zriver_output_status_v1::Event::ViewTags {
                            tags,
                        } => {
                            viewstag = tags[0..]
                                .chunks(4)
                                .map(|s| {
                                    let buf = [s[0], s[1], s[2], s[3]];
                                    u32::from_le_bytes(buf)
                                })
                                .collect();
                        }
                        zriver_output_status_v1::Event::UrgentTags{ tags } => {
                        	// app.widget.send_command(Command::Data("urgent", &tags), &mut Vec::new(), 0, 0);
                        }
                    }
                }
            });
        }
	}
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

fn create_widget(amount: u32, icon_size: u32) -> Border<Background<WidgetLayout>> {
    let mut tags = WidgetLayout::new(Orientation::Horizontal);
    tags.set_spacing(10);

    for n in 0..amount {
        let tag = 1 << n;
        tags.add(modules::TagButton::new(tag, icon_size)).unwrap();
    }

    boxed(tags, 10, 1, BG0, BG1)
}
