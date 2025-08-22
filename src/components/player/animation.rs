use bevy::animation::AnimationPlayer;
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::{Added, AnimationGraph, AnimationGraphHandle, AnimationNodeIndex, ChildOf, Commands, Component, Entity, Local, Query, Res, ResMut, Resource, With};
use serde::{Deserialize, Serialize};
use crate::components::player::PlayerMarker;

#[derive(Resource)]
pub struct PlayerAnimationGraph(Handle<AnimationGraph>);

#[derive(Component)]
pub struct AnimationPlayerLink(pub Entity);

#[derive(Component)]
pub struct PlayerAnimationState(pub AnimationState);

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq)]
pub enum AnimationState {
    #[default]
    Idle,
    Walking,
}

pub fn get_top_parent(
    mut curr_entity: Entity,
    all_entities_with_parents_query: &Query<&ChildOf>,
) -> Entity {
    //Loop up all the way to the top parent
    loop {
        if let Ok(ref_to_parent) = all_entities_with_parents_query.get(curr_entity) {
            curr_entity = ref_to_parent.0;
        } else {
            break;
        }
    }
    curr_entity
}

pub fn setup_player_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut animation_graph = AnimationGraph::new();
    animation_graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(0).from_asset("meshes\\player.glb")),
        1.0,
        animation_graph.root
    );
    animation_graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(1).from_asset("meshes\\player.glb")),
        1.0,
        animation_graph.root
    );

    let anim_graph_handle = animation_graphs.add(animation_graph);
    commands.insert_resource(PlayerAnimationGraph(anim_graph_handle));
}

pub fn player_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    mut all_parents_query: Query<&ChildOf>,
    animation_graph: Res<PlayerAnimationGraph>,
    mut done: Local<bool>
) {
    // if *done {
    //     return;
    // }
    for (entity, mut player) in query.iter_mut() {
        println!("Animation Player Found");

        commands.entity(entity).insert((
            AnimationGraphHandle(animation_graph.0.clone()),
        ));
        for i in 1..3 {
            player.play(AnimationNodeIndex::new(i)).repeat();
        }

        let top_entity = get_top_parent(entity, &mut all_parents_query);
        commands.entity(top_entity).log_components();
        commands.entity(top_entity).insert(AnimationPlayerLink(entity));

        // *done = true;
    }
}

pub fn animation_control(
    mut commands: Commands,
    mut animation_players: Query<&mut AnimationPlayer>,
    mut player_anim_state: Query<(&mut PlayerAnimationState, &AnimationPlayerLink), With<PlayerMarker>>,
) {
    for player_state in player_anim_state.iter_mut() {
        if let Some(mut anim_play) = animation_players.get_mut(player_state.1.0).ok() {
            match player_state.0.0 {
                AnimationState::Idle => {
                    if let Some(idle_anim) = anim_play.animation_mut(AnimationNodeIndex::new(1)) {
                        idle_anim.set_weight(1.0);
                    }
                    if let Some(walking_anim) = anim_play.animation_mut(AnimationNodeIndex::new(2)) {
                        walking_anim.set_weight(0.0);
                    }
                }
                AnimationState::Walking => {
                    if let Some(idle_anim) = anim_play.animation_mut(AnimationNodeIndex::new(1)) {
                        idle_anim.set_weight(0.0);
                    }
                    if let Some(walking_anim) = anim_play.animation_mut(AnimationNodeIndex::new(2)) {
                        walking_anim.set_weight(1.0);
                    }
                }
            }
        }
    }
}