use bevy::prelude::*;

use crate::GameState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameState::Menu), setup_menu)
            .add_systems(Update, handle_buttons.run_if(in_state(GameState::Menu)))
            .add_systems(OnExit(GameState::Menu), cleanup_menu);
    }
}

#[derive(Component)]
struct MenuUI;

#[derive(Component)]
enum MenuButton {
    Play,
    Credits,
}

fn setup_menu(mut commands: Commands, assets: Res<AssetServer>) {
    // Root canvas
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(1280.0),
                height: Val::Px(720.0),
                ..default()
            },
            ZIndex(100), // on top of world
            MenuUI,
        ))
        .with_children(|root| {
            // Background
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(1280.0),
                    height: Val::Px(720.0),
                    ..default()
                },
                ImageNode::new(assets.load("menu/Title_BG.png")),
            ));

            // Cleanup Crew Title
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(1280.0),
                    height: Val::Px(720.0),
                    ..default()
                },
                ImageNode::new(assets.load("menu/Title_Text.png")),
            ));

            root
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(80.0),
                        ..default()
                    },
                ))
                .with_children(|col| {
                    // Play
                    col.spawn((
                        Button,
                        MenuButton::Play,
                        ImageNode::new(assets.load("menu/Title_Play.png")),
                    ));

                    // Credits
                    col.spawn((
                        Button,
                        MenuButton::Credits,
                        ImageNode::new(assets.load("menu/Title_Credits.png")),
                    ));
                });
        });
}

fn handle_buttons(
    mut interactions: Query<(&Interaction, &MenuButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, which) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match which {
            MenuButton::Play => next_state.set(GameState::Loading),
            MenuButton::Credits => next_state.set(GameState::EndCredits),
        }
    }
}

fn cleanup_menu(mut commands: Commands, root_q: Query<Entity, With<MenuUI>>) {
    for e in &root_q {
        commands.entity(e).despawn_recursive();
    }
}