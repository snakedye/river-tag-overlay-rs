mod wayland;
mod environment;

use snui::*;
use snui::widgets::*;
use snui::wayland::app;
use snui::wayland::app::Shell;

const FG: u32 = 0xff_B5_B1_A4;
const BG0: u32 = 0xff_26_25_25;
const BG1: u32 = 0xff_33_32_32;
const YEL: u32 = 0xff_c6_aa_82;
const GRN: u32 = 0xff_98_96_7E;

use environment::Environment;
use wayland_client::{Display, Attached};
use smithay_client_toolkit::shm::AutoMemPool;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;

use crate::wayland::river_status_unstable_v1::zriver_output_status_v1;
use crate::wayland::river_status_unstable_v1::zriver_seat_status_v1 ;

fn main() {
    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let environment = Environment::new(&display, &mut event_queue);
    let attached = Attached::from(environment.shm.clone().expect("No shared memory pool"));

    let layer_shell = environment.layer_shell.as_ref().unwrap();
    let status_manager = environment.status_manager.as_ref().unwrap();

    let mut applications: Vec<app::Application> = environment.outputs.iter().enumerate().map(|(i, output)| {
        let surface = environment.get_surface().detach();
        let mempool = AutoMemPool::new(attached.clone()).unwrap();
        let layer_surface = layer_shell
            .get_layer_surface(&surface, Some(&output.wl_output), Layer::Top, String::from("overlay"));

        let mut app = app::Application::new(
            create_widget(10, output.width as u32),
            surface.clone(),
            mempool,
        );
        app.attach_layer_surface(&layer_surface);

        let mut tag_list = Vec::new();
        let output_status = status_manager.get_river_output_status(&output.wl_output);
        output_status.quick_assign(move |_, event, mut applications | {
            match event {
                zriver_output_status_v1::Event::FocusedTags { tags } => {
                    let application = &mut applications.get::<Vec<app::Application>>().unwrap()[i];
                    application.dispatch(Command::Data("occupied", &tag_list));
                	application.dispatch(Command::Data("focused", &tags));
                }
                zriver_output_status_v1::Event::ViewTags { tags } => {
                    let len = tags.len();
                    for _ in applications.get::<Vec<app::Application>>().unwrap() {
                        tag_list = (0..len).into_iter().step_by(4).map(|i| {
                            let buf: [u8; 4] = [tags[i], tags[i + 1], tags[i + 2], tags[i + 3]];
                            u32::from_le_bytes(buf)
                        }).collect();
                    }
                }
            }
        });
        app
	}).collect();

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

fn run_command(value: String) {
    use std::process::Command;
    let mut string = value.split_whitespace();
    let mut command = Command::new(string.next().unwrap());
    command.args(string.collect::<Vec<&str>>());
    command.spawn().expect("Error");
}

fn create_widget(amount: u32, width: u32) -> Border<Background<Rectangle, WidgetLayout>> {
    let icon_size = 30;

    let mut tags = WidgetLayout::new(Orientation::Horizontal);
    tags.set_spacing(5);

    for n in 0..amount {
        let tag = 1 << n;
        let icon = Background::new(Rectangle::square(20, BG1), Rectangle::square(0,BG1), icon_size - 4);
        let action = Actionnable::new(icon, move |icon, action| {
            if action.eq("focused") {
                if let Some(focused) = action.get::<u32>() {
                    if focused == &tag || (focused / tag) % 2 != 0 {
                        icon.widget.set_color(YEL);
                        return true
                    }
                }
            } else if action.eq("occupied") {
                if let Some(tags) = action.get::<Vec<u32>>() {
                    icon.widget.set_color(BG1);
                    for t in tags {
                        if t == &tag {
                            icon.widget.set_color(GRN);
                            return true
                        }
                    }
                }
            } else if action.eq("title") {
                return true
            }
            false
        });
        tags.add(Button::new(action, move |_, _, _, _, _, ev| match ev {
	        pointer::Event::MouseClick {
                time: _,
                button: _,
                pressed,
            } => {
                if pressed {
                    run_command(format!("riverctl set-focused-tags {}", tag));
                }
                Damage::None
            }
            _ => Damage::None,
        })).unwrap();
    }

    boxed(tags, 5, 1, BG0, BG0)
}
