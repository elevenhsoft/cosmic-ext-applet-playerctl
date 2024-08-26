use std::time::Duration;

use cosmic::{
    app::{Command, Core},
    iced::{subscription, Alignment, Length, Pixels, Subscription},
    iced_core::text::LineHeight,
    iced_style::application,
    iced_widget::row,
    widget::{container, vertical_space},
    Application, Element, Theme,
};

use crate::player::{run, MprisUpdate};

#[derive(Debug, Clone)]
pub enum Message {
    UpdateTrack(MprisUpdate),
}

pub struct Window {
    core: Core,
    formatted_track: String,
}

impl Application for Window {
    type Executor = cosmic::SingleThreadExecutor;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "io.github.elevenhsoft.CosmicExtAppletPlayerctl";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Message>) {
        let formatted_track = String::new();

        (
            Self {
                core,
                formatted_track,
            },
            Command::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::channel(0, 50, move |mut output| async move {
            loop {
                run(&mut output).await;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        })
        .map(Message::UpdateTrack)
    }

    fn style(&self) -> Option<<Theme as application::StyleSheet>::Style> {
        Some(cosmic::applet::style())
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::UpdateTrack(mpris) => {
                let MprisUpdate::Player(player) = mpris;

                if let Some(status) = player.get_status() {
                    let artist = match status.artists {
                        Some(artists) => artists.concat(),
                        None => String::new(),
                    };
                    let title = status.title.unwrap_or_default();

                    self.formatted_track = format!("{} - {}", artist, title);
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
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
            .style(cosmic::theme::Button::AppletIcon);

        container(button).max_width(300.0).into()
    }
}
