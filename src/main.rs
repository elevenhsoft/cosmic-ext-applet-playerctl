use window::Window;

mod player;
mod window;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Window>(true, ())
}
