mod components;
mod network;
mod test;

use crate::components::chat::{Chat, chat_window};
use crate::components::hud::Hud;
use crate::components::player::{PlayerInfo, player_controller, PlayerMarker, update_label_pos};
use crate::network::net_manage::{
    Communication, TcpConnection,
};
use crate::network::net_message::{NetworkMessage, CTcpType};
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::{ResourceInspectorPlugin, WorldInspectorPlugin};
use std::collections::VecDeque;
use std::io;
use avian3d::PhysicsPlugins;
use avian3d::prelude::{Collider, CollisionLayers, LayerMask, LinearVelocity, Physics, PhysicsDebugPlugin, PhysicsTime, RigidBody, Sleeping};
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::text::FontSmoothing;
use crate::components::camera::camera_controller;
use crate::components::CollisionLayer;
use crate::components::common::Id;
use crate::components::player::animation::{animation_control, player_animations, setup_player_animations};
use crate::components::player::plugin::PlayerPlugin;
use crate::components::weapon::{weapon_controller, Weapon};
use crate::network::{NetworkPlugin, RemoteAddress};

#[derive(Resource)]
pub struct DefaultFont(pub Handle<Font>);

const LOBBY_ID: u32 = 1;
fn join_lobby(
    mut keyboard_input: EventReader<KeyboardInput>,
    mut connection: ResMut<TcpConnection>,
) {
    for k in keyboard_input.read() {
        if k.state == ButtonState::Released {
            continue;
        };

        match k.key_code {
            KeyCode::KeyJ => {
                if connection.stream.is_some() {
                    connection.add_message(NetworkMessage(CTcpType::Join { lobby_id: Id(LOBBY_ID) }));
                }
            }
            _ => {}
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let default_address = "127.0.0.1:4444".to_string();
    let remote_address = args.get(1).unwrap_or(&default_address);
    
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        PhysicsPlugins::default().with_length_unit(10.0),
        EguiPlugin::default(),
        WorldInspectorPlugin::new(),
        ResourceInspectorPlugin::<PlayerInfo>::default(),
        FpsOverlayPlugin::default(),
        // PhysicsDebugPlugin::default(),
        NetworkPlugin,
        PlayerPlugin
    ));
    app.insert_resource(Time::<Fixed>::from_hz(60.0));
    app.insert_resource(Time::<Physics>::default().with_relative_speed(1.0));
    app.insert_resource(DefaultFont(Handle::default()));
    app.insert_resource(RemoteAddress(remote_address.clone()));
    app.add_systems(Startup, setup);
    app.add_systems(
        FixedUpdate,
        (
            join_lobby,
            chat_window,
            // debug_player_sleeping
            // linear_is_changed
        )
    );
    app.run();

    Ok(())
}

fn setup(
    mut default_font: ResMut<DefaultFont>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    default_font.0 = asset_server.load("fonts\\alagard.ttf");
    
    println!("{:?}", default_font.0);
    
    // Main Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y)
    ));

    // Ground Plane
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(40.0, 0.5, 40.0),
        CollisionLayers::new(CollisionLayer::Ground, [LayerMask::ALL]),
        Mesh3d(meshes.add(Cuboid::new(40.0,0.5,40.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    //Light Source
    commands.spawn((
        PointLight {
            intensity: 10000000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 0.0)
    ));

    //Position and ID Hud
    commands.spawn((
        Hud,
        Text::new(""),
        TextFont {
            font: default_font.0.clone(),
            font_size: 20.0,
            line_height: Default::default(),
            font_smoothing: FontSmoothing::None,
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.5),
            right: Val::Px(0.5),
            ..default()
        },
    ));

    // Chat Window
    commands.spawn((
        Chat {
            chat_history: VecDeque::new(),
        },
        Text::new(""),
        TextFont {
            font: default_font.0.clone(),
            font_size: 20.0,
            line_height: Default::default(),
            font_smoothing: FontSmoothing::None,
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.5),
            left: Val::Px(0.5),
            ..default()
        },
    ));

    commands.spawn(Weapon{ damage: 10, range: 100.0 });
}

fn linear_is_changed(
    id: Query<&Id, Changed<LinearVelocity>>,
) {
    for id in id.iter() {
        println!("player linear velo changed: {:?}", id);
    }
}

fn debug_player_sleeping(
    sleeping_players: Query<(&LinearVelocity, &PlayerMarker), With<Sleeping>>,
    nonsleeping_players: Query<(&LinearVelocity, &PlayerMarker), Without<Sleeping>>,
) {
    for p in sleeping_players.iter() {
        println!("Sleeping: {:?}", p.0);
    }
    
    for p in nonsleeping_players.iter() {
        println!("NonSleeping: {:?}", p.0);
    }
}
