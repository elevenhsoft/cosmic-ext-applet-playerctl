use cosmic::iced::futures::{self, future::OptionFuture, SinkExt, StreamExt};
use mpris2_zbus::{enumerator, player::Player};
use std::path::PathBuf;
use urlencoding::decode;
use zbus::{
    names::{BusName, OwnedBusName},
    Connection,
};

#[derive(Clone, Debug)]
pub struct PlayerStatus {
    pub artists: Option<Vec<String>>,
    pub title: Option<String>,
}

impl PlayerStatus {
    pub async fn new(player: Player) -> Option<Self> {
        let metadata = player.metadata().await.ok()?;
        let pathbuf = PathBuf::new();

        let title = metadata.title().or(pathbuf
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| decode(s).map_or(None, |s| Some(s.into_owned()))));
        let artists = metadata
            .artists()
            .map(|a| a.into_iter().collect::<Vec<_>>());

        Some(Self { artists, title })
    }
}

#[derive(Clone, Debug)]
pub enum MprisUpdate {
    Status(PlayerStatus),
    Finished,
}

#[derive(Clone, Debug)]
pub struct MprisPlayer {
    player: Player,
}

impl MprisPlayer {
    async fn new(conn: &Connection, name: OwnedBusName) -> mpris2_zbus::error::Result<Self> {
        let player = Player::new(conn, name.clone()).await?;

        Ok(Self {
            player: player.clone(),
        })
    }

    fn name(&self) -> &BusName {
        self.player.inner().destination()
    }
}

pub struct State {
    players: Vec<MprisPlayer>,
    player: Option<MprisPlayer>,
    active_player_metadata_stream: Option<Box<dyn futures::Stream<Item = ()> + Unpin + Send>>,
}

impl State {
    pub async fn new() -> Result<Self, zbus::Error> {
        let conn = Connection::session().await?;
        let enumerator = enumerator::Enumerator::new(&conn).await?;
        let player_names = enumerator.players().await?;

        let mut players = Vec::with_capacity(player_names.len());

        for name in player_names.into_iter() {
            if let Ok(mpris) = MprisPlayer::new(&conn, name).await {
                players.push(mpris);
            }
        }

        players.sort_by(|a, b| a.name().cmp(b.name()));

        let mut state = State {
            players: players.clone(),
            player: get_active_player(players).await,
            active_player_metadata_stream: None,
        };

        state.get_metadata_stream().await;

        Ok(state)
    }

    pub async fn get_metadata_stream(&mut self) {
        if let Some(player) = get_active_player(self.players.clone()).await {
            let controls_changed = futures::stream::select_all([
                player.player.receive_can_pause_changed().await,
                player.player.receive_can_play_changed().await,
                player.player.receive_can_go_previous_changed().await,
                player.player.receive_can_go_next_changed().await,
            ]);
            let metadata_changed = player.player.receive_metadata_changed().await;

            let stream =
                futures::stream::select(controls_changed.map(|_| ()), metadata_changed.map(|_| ()));

            self.active_player_metadata_stream = Some(Box::new(stream));
        }
    }
}

pub async fn get_active_player(players: Vec<MprisPlayer>) -> Option<MprisPlayer> {
    let mut best = (0, None::<MprisPlayer>);
    let eval = |p: Player| async move {
        let v = {
            let status = p.playback_status().await;

            match status {
                Ok(mpris2_zbus::player::PlaybackStatus::Playing) => 100,
                Ok(mpris2_zbus::player::PlaybackStatus::Paused) => 10,
                _ => return 0,
            }
        };

        v + p.metadata().await.is_ok() as i32
    };

    for p in players.iter() {
        let v = eval(p.player.clone()).await;
        if v > best.0 {
            best = (v, Some(p.to_owned()));
        }
    }

    best.1
}

pub async fn run(output: &mut futures::channel::mpsc::Sender<MprisUpdate>) {
    let mut state = match State::new().await {
        Ok(state) => state,
        Err(err) => {
            println!("Error: {}", err);
            return;
        }
    };

    loop {
        if let Some(player) = &state.player {
            if let Some(status) = PlayerStatus::new(player.player.clone()).await {
                let _ = output.send(MprisUpdate::Status(status)).await;
            }
        };

        let metadata_changed_next = OptionFuture::from(
            state
                .active_player_metadata_stream
                .as_mut()
                .map(|s| s.next()),
        );

        tokio::select! {
            _ = metadata_changed_next, if state.player.is_some() => {}
        };
    }
}
