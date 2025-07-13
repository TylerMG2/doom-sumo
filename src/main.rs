// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

//mod asset_tracking;

use bevy::{asset::AssetMetaCheck, platform::collections::HashMap, prelude::*};
use bevy_ggrs::*;
use bevy_matchbox::{MatchboxSocket, matchbox_socket::RtcIceServerConfig, prelude::*};

fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // Add Bevy plugins.
        app.add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    // This causes errors and even panics on web build on itch.
                    // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Doom Sumo".to_string(),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                }),
        );

        app.add_plugins((GgrsPlugin::<Config>::default(),))
            .rollback_component_with_clone::<Transform>()
            .add_systems(ReadInputs, read_local_inputs)
            .add_systems(GgrsSchedule, move_players);

        // Spawn the main camera.
        app.add_systems(Update, wait_for_players);

        app.add_systems(
            Startup,
            (spawn_camera, spawn_players, start_matchbox_socket),
        );
    }
}

#[derive(Component)]
struct Player {
    handle: usize,
}

fn spawn_players(mut commands: Commands) {
    // Player 1
    commands
        .spawn((
            Player { handle: 0 },
            Transform::from_translation(Vec3::new(-2., 0., 0.)), // changed
            Sprite {
                color: Color::srgb(0., 0.47, 1.),
                custom_size: Some(Vec2::new(64., 64.)),
                ..default()
            },
        ))
        .add_rollback();

    // Player 2
    commands
        .spawn((
            Player { handle: 1 },
            Transform::from_translation(Vec3::new(2., 0., 0.)),
            Sprite {
                color: Color::srgb(0., 0.4, 0.),
                custom_size: Some(Vec2::new(64., 64.)),
                ..default()
            },
        ))
        .add_rollback();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Name::new("Camera"), Camera2d));
}

fn move_players(
    mut players: Query<(&mut Transform, &Player)>,
    inputs: Res<PlayerInputs<Config>>,
    time: Res<Time>,
) {
    for (mut transform, player) in &mut players {
        let (input, _) = inputs[player.handle];

        let mut direction = Vec2::ZERO;

        if input & INPUT_UP != 0 {
            direction.y += 1.;
        }
        if input & INPUT_DOWN != 0 {
            direction.y -= 1.;
        }
        if input & INPUT_RIGHT != 0 {
            direction.x += 1.;
        }
        if input & INPUT_LEFT != 0 {
            direction.x -= 1.;
        }
        if direction == Vec2::ZERO {
            continue;
        }

        let move_speed = 70.;
        let move_delta = direction * move_speed * time.delta_secs();
        transform.translation += move_delta.extend(0.);
    }
}

// Netcode
fn start_matchbox_socket(mut commands: Commands) {
    let room_url = "ws://38.179.101.216:3536/doom_sumo?next=2";
    info!("connecting to matchbox server: {room_url}");
    commands.insert_resource(MatchboxSocket::from(
        WebRtcSocketBuilder::new(room_url)
            .ice_server(RtcIceServerConfig {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                username: None,
                credential: None,
            })
            .add_channel(ChannelConfig::unreliable())
            .build(),
    ));
}

type Config = bevy_ggrs::GgrsConfig<u8, PeerId>;

fn wait_for_players(mut commands: Commands, mut socket: ResMut<MatchboxSocket>) {
    if socket.get_channel(0).is_err() {
        return; // we've already started
    }

    // Check for new connections
    socket.update_peers();
    let players = socket.players();

    let num_players = 2;
    if players.len() < num_players {
        return; // wait for more players
    }

    info!("All peers have joined, going in-game");

    // create a GGRS P2P session
    let mut session_builder = ggrs::SessionBuilder::<Config>::new()
        .with_num_players(num_players)
        .with_input_delay(2);

    for (i, player) in players.into_iter().enumerate() {
        session_builder = session_builder
            .add_player(player, i)
            .expect("failed to add player");
    }

    // move the channel out of the socket (required because GGRS takes ownership of it)
    let channel = socket.take_channel(0).unwrap();

    // start the GGRS session
    let ggrs_session = session_builder
        .start_p2p_session(channel)
        .expect("failed to start session");

    commands.insert_resource(bevy_ggrs::Session::P2P(ggrs_session));
}

// Inputs
const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;

fn read_local_inputs(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    local_players: Res<LocalPlayers>,
) {
    let mut local_inputs = HashMap::new();

    for handle in &local_players.0 {
        let mut input = 0u8;

        if keys.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
            input |= INPUT_UP;
        }
        if keys.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
            input |= INPUT_DOWN;
        }
        if keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
            input |= INPUT_LEFT
        }
        if keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
            input |= INPUT_RIGHT;
        }

        local_inputs.insert(*handle, input);
    }

    commands.insert_resource(LocalInputs::<Config>(local_inputs));
}
