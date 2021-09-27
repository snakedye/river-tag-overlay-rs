mod wayland;

use snui::*;
use std::time;
use std::thread;
use snui::widgets::*;
use snui::wayland::app;
use std::sync::mpsc::Sender;
use wayland_client::{Display, Proxy};
use crate::wayland::river_status_unstable_v1::zriver_output_status_v1;
use crate::wayland::river_status_unstable_v1::zriver_status_manager_v1::ZriverStatusManagerV1;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_surface_v1;

use wayland_client::protocol::{
    wl_compositor::WlCompositor, wl_output::WlOutput, wl_shm::WlShm, wl_surface::WlSurface,
};

use smithay_client_toolkit::{
    environment,
    environment::{Environment, SimpleGlobal},
    output::OutputHandler,
    shm::ShmHandler,
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
const GRN: u32 = 0xff_98_96_7E;
const YEL: u32 = 0xff_c6_aa_82;

struct TagsData {
    focused: u32,
    views: Vec<u32>,
}

fn main() {
    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let wl_display = Proxy::clone(&display).attach(event_queue.token());
    let env = Environment::new(&wl_display, &mut event_queue, Env::new()).unwrap();

    let widget = create_widget(9, 40);
    let mut mempool = env.create_auto_pool().unwrap();
    let shm = env.require_global::<WlShm>();
    let status_manager = env.require_global::<ZriverStatusManagerV1>();
    let draw = mempool
        .resize((widget.get_width() * widget.get_height() * 4) as usize)
        .is_ok();

    let surface = env.create_surface();
	let display_handle = display.clone();
    let (app, mut sender) = app::Application::new(widget, surface.detach(), shm.detach());
    thread::spawn(|| {
        let mut state = 0;
        app.run(display_handle, |app, pool, dispatch| match dispatch {
                Dispatch::Data(name, mut data) => match name {
                    "tagdata" => if let Some(tags) = data.as_mut().downcast_mut::<TagsData>() {
                        for w in &mut app.widget.widget.widget.widgets.iter_mut() {
                            w.widget.set_color(BG1);
                        }
                        for t in &tags.views {
                            if *t < 9 {
                                app.widget.widget.widget.widgets[*t as usize].widget.set_color(GRN);
                            }
                        }
                        for (i, w) in &mut app.widget.widget.widget.widgets.iter_mut().enumerate() {
                            let tagmask = 1 << i;
                            if tagmask == tags.focused {
                                w.widget.set_color(YEL);
                            } else if tagmask != tags.focused && (tags.focused / tagmask) % 2 != 0 {
                                w.widget.set_color(YEL);
                                tags.focused -= 1 << i;
                            }
                        }
                    },
                    "swap" => {
                        if let Some((surface, layer_surface)) = data.as_ref()
                        	.downcast_ref::<(WlSurface, zwlr_layer_surface_v1::ZwlrLayerSurfaceV1)>() {
                                if state == 0 {
                                	app.destroy();
                                	app.surface = surface.clone();
                                	app.layer_surface = Some(layer_surface.clone());
                                    layer_surface.set_size(app.widget.get_width(), app.widget.get_height());
                                    surface.commit();
                                } else {
                                    app.render(pool);
                                    app.show();
                                }
                                state += 1;
                        }
                    }
                    _ => {}
                }
                Dispatch::Message(msg) => if msg == "hide" {
                    if state > 1 {
                        state -= 1;
                    } else {
                        app.hide();
                        state -= 1;
                    }
                }
                Dispatch::Commit => if state > 0 {
                    app.init(pool);
                }
                _ => {}
            }
        );
    });

    if draw {
        for output in env.get_all_outputs() {
            let compositor = env.require_global::<WlCompositor>();
            let layer_shell = env.require_global::<ZwlrLayerShellV1>();
            let output_status = status_manager.get_river_output_status(&output);

            let mut viewstag: Vec<u32> = Vec::new();
            output_status.quick_assign(move |_, event, mut sender| {
                if let Some(sender) = sender.get::<Sender<Dispatch>>() {
                    match event {
                        zriver_output_status_v1::Event::FocusedTags { tags } => {
                            let tagdata = TagsData {
                                focused: tags,
                                views: viewstag.clone()
                            };
                            if sender.send(Dispatch::Data("tagdata", Box::new(tagdata))).is_ok() {
                                let surface = compositor.create_surface();
                                let layer_surface = layer_shell
                                    .get_layer_surface(&surface, None, Layer::Overlay, "overlay".to_owned());
                                surface.quick_assign(|_, _, _| {});
                                app::assign_layer_surface(&surface, &layer_surface);
                                sender.send(Dispatch::Data("swap", Box::new((surface.detach(), layer_surface.detach())))).unwrap();
                                let handle = sender.clone();
                                thread::spawn(move || {
                                    thread::sleep(time::Duration::from_millis(500));
                                    if let Err(e) = handle.send(Dispatch::Message("hide")) {
                                        eprintln!("{}", e);
                                    }
                                });
                            }
                        }
                        zriver_output_status_v1::Event::ViewTags { tags } => {
                            viewstag = tags[0..]
                                .chunks(4)
                                .map(|s| {
                                    let buf = [s[0], s[1], s[2], s[3]];
                                    u32::from_le_bytes(buf)
                                })
                                .collect();
                        }
                        zriver_output_status_v1::Event::UrgentTags { tags } => {
                            sender.send(Dispatch::Data("urgent", Box::new(tags))).unwrap();
                        }
                    }
                }
            });
        }
    }

    loop {
        event_queue
            .dispatch(&mut sender, |event, object, _| {
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
    let mut tags = WidgetLayout::horizontal(10);

    for _ in 0..amount {
        tags.add(Rectangle::square(icon_size, BG1)).unwrap();
    }

    boxed(tags, 10, 1, BG0, BG1)
}
