use cosmic::iced::futures::{self, SinkExt};
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
    Player(MprisPlayer),
}

#[derive(Clone, Debug)]
pub struct MprisPlayer {
    player: Player,
    status: Option<PlayerStatus>,
}

impl MprisPlayer {
    async fn new(conn: &Connection, name: OwnedBusName) -> mpris2_zbus::error::Result<Self> {
        let player = Player::new(conn, name.clone()).await?;

        Ok(Self {
            player: player.clone(),
            status: PlayerStatus::new(player).await,
        })
    }

    fn name(&self) -> &BusName {
        self.player.inner().destination()
    }

    pub fn get_status(self) -> Option<PlayerStatus> {
        self.status
    }
}

pub struct State {
    players: Vec<MprisPlayer>,
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

        Ok(Self { players })
    }

    pub async fn get_active_player(&self) -> Option<MprisPlayer> {
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

        for p in self.players.iter() {
            let v = eval(p.player.clone()).await;
            if v > best.0 {
                best = (v, Some(p.to_owned()));
            }
        }

        best.1
    }
}

pub async fn run(output: &mut futures::channel::mpsc::Sender<MprisUpdate>) {
    let state = match State::new().await {
        Ok(state) => state,
        Err(err) => {
            println!("Error: {}", err);
            return;
        }
    };

    if let Some(mpris_player) = state.get_active_player().await {
        let _ = output.send(MprisUpdate::Player(mpris_player)).await;
    }
}
