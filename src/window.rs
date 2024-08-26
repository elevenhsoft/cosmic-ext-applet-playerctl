use std::time::Duration;

use cosmic::{
    app::Core,
    iced::{subscription, Alignment, Length, Pixels, Subscription},
    iced_core::text::LineHeight,
    iced_style::application,
    iced_widget::row,
    widget::{container, vertical_space},
    Application, Command, Element, Theme,
};

use playerctl::PlayerCtl;

#[derive(Debug, Clone)]
pub enum Message {
    UpdateTrack,
}

pub struct Window {
    core: Core,
    formatted_track: String,
}

impl Application for Window {
    type Executor = cosmic::SingleThreadExecutor;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "io.github.elevenhsoft.CosmicExtPlayerctlMetadata";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, cosmic::app::Command<Self::Message>) {
        let formatted_track = String::new();

        (
            Self {
                core,
                formatted_track,
            },
            Command::none(),
        )
    }

    fn style(&self) -> Option<<Theme as application::StyleSheet>::Style> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let subscription = subscription::unfold("refresh", (), move |()| async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            ((), ())
        })
        .map(|_| Message::UpdateTrack);

        Subscription::batch(vec![subscription])
    }

    fn update(&mut self, message: Self::Message) -> cosmic::app::Command<Self::Message> {
        match message {
            Message::UpdateTrack => {
                let metadata = PlayerCtl::metadata();

                self.formatted_track = format!("{} - {}", &metadata.artist, &metadata.title);
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        let length = (self.core.applet.suggested_size(true).0
            + 2 * self.core.applet.suggested_padding(true)) as f32;

        let fixed_length = Length::Fixed(length);

        let song_text = cosmic::widget::Text::new(&self.formatted_track)
            .line_height(LineHeight::Absolute(Pixels::from(length)));

        let ele = Element::from(
            row!(song_text, container(vertical_space(fixed_length))).align_items(Alignment::Center),
        );

        let button = cosmic::widget::button(ele)
            .padding([0, self.core.applet.suggested_padding(true)])
            .style(cosmic::theme::Button::AppletIcon)
            .on_press(Message::UpdateTrack);

        container(button).max_width(300.0).into()
    }
}
