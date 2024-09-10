#![allow(unused_imports)]
use bevy::prelude::*;
use bevy_cleancut::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_particle_systems::ParticleSystemPlugin;
use bevy_rapier2d::prelude::*;
use bevy_scoreboard::prelude::*;
use leafwing_input_manager::prelude::*;
use rand::{thread_rng, Rng};
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::Space)))
        .add_plugins(RapierPhysicsPlugin::<()>::pixels_per_meter(200.0))
        .add_plugins(RapierDebugRenderPlugin::default().disabled())
        .add_plugins(ParticleSystemPlugin)
        .add_plugins(InputManagerPlugin::<Action>::default())
        .add_plugins(ScoreboardPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (spawn_star, cleanup_star, join, movement, catch_star),
        )
        .insert_resource(StarTimer(Timer::from_seconds(0.0, TimerMode::Repeating)))
        .run();
}

fn catch_star(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut scoreboard: ResMut<Scoreboard>,
    mut collision_events: EventReader<CollisionEvent>,
    player_qry: Query<&Player>,
    star_qry: Query<&Star>,
    transform_qry: Query<&Transform, With<Star>>,
) {
    for event in collision_events.read() {
        if let Some((player_entity, star_entity)) = collision_started(&player_qry, &star_qry, event)
        {
            play_sound(&mut commands, &asset_server, "bom.wav", 0.25);

            // Increment the player's score
            let player = player_qry.get(player_entity).unwrap();
            scoreboard.increment(player.id, 1);

            // Did the player win?
            if scoreboard.get_score(player.id) >= 5 {
                scoreboard.show_winner_screen(player_entity);
            }

            // Despawn star
            commands.entity(star_entity).despawn_recursive();

            // Particle poof!
            spawn_particle_poof(&mut commands, transform_qry.get(star_entity).unwrap());
        }
    }
}

fn movement(
    mut action_query: Query<(
        &ActionState<Action>,
        &mut Velocity,
        &mut ExternalForce,
        &Transform,
    )>,
) {
    for (action, mut velocity, mut external_force, transform) in &mut action_query {
        // Run
        external_force.force = Vec2::new(action.value(&Action::Run) * 1000.0, 0.0);

        // Jump
        if action.just_pressed(&Action::Jump) && transform.translation.y < -315.0 {
            velocity.linvel.y = 1250.0;
        }
    }
}

fn join(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut scoreboard: ResMut<Scoreboard>,
    mut gamepad_connection_events: EventReader<GamepadConnectionEvent>,
) {
    for event in gamepad_connection_events.read() {
        if event.disconnected() {
            continue;
        }

        // Set up input
        let input_map = InputMap::default()
            .with(Action::Jump, GamepadButtonType::South)
            .with_axis(
                Action::Run,
                GamepadControlAxis::LEFT_X.with_deadzone_symmetric(0.1),
            )
            .with_gamepad(event.gamepad);

        // Spawn player
        let id = event.gamepad.id;
        let color = PlayerColors::index(id);
        let name = format!("Ferris{}", id);
        scoreboard.add_player(id, &name, color);
        commands.spawn((
            Name::new(name),
            Player { id },
            InputManagerBundle::with_map(input_map),
            SpriteBundle {
                sprite: Sprite {
                    color,
                    ..Default::default()
                },
                transform: Transform::from_xyz(thread_rng().gen_range(-600.0..600.0), -250.0, 2.0),
                texture: asset_server.load("ferris.png"),
                ..Default::default()
            },
            RigidBody::Dynamic,
            Damping {
                linear_damping: 3.0,
                angular_damping: 0.0,
            },
            Restitution::new(1.5),
            Collider::ball(28.0),
            Velocity::default(),
            ExternalForce::default(),
            ColliderMassProperties::Mass(1.0),
        ));
    }
}

#[derive(Component)]
struct Player {
    id: usize,
}

fn cleanup_star(mut commands: Commands, star_qry: Query<(Entity, &Transform), With<Star>>) {
    for (star_entity, transform) in &star_qry {
        if transform.translation.y < -400.0 {
            commands.entity(star_entity).despawn_recursive();
        }
    }
}

fn spawn_star(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut star_timer: ResMut<StarTimer>,
    time: Res<Time>,
) {
    let mut rng = thread_rng();
    if star_timer.0.tick(time.delta()).just_finished() {
        // Set timer for next star
        let duration = Duration::from_secs_f32(rng.gen_range(0.2..2.0));
        star_timer.0.set_duration(duration);

        play_sound(&mut commands, &asset_server, "bling.wav", 0.25);

        // Spawn the star
        commands
            .spawn((
                Name::new("Star"),
                Star,
                SpriteBundle {
                    texture: asset_server.load("star.png"),
                    transform: Transform::from_xyz(rng.gen_range(-400.0..400.0), 350.0, 1.0),
                    ..Default::default()
                },
                RigidBody::Dynamic,
                ActiveEvents::COLLISION_EVENTS,
                Sensor,
                Collider::ball(28.0),
                GravityScale(0.0),
                Velocity {
                    linvel: Vec2::new(rng.gen_range(-75.0..75.0), rng.gen_range(-300.0..-100.0)),
                    angvel: rng.gen_range(-1.2..1.2),
                },
            ))
            .with_children(|builder| {
                builder.spawn(particle_trail_bundle());
            });
    }
}

#[derive(Component)]
struct Star;

#[derive(Resource)]
struct StarTimer(Timer);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Background
    commands.spawn((
        Name::new("Background"),
        SpriteBundle {
            texture: asset_server.load("starry-field.jpg"),
            transform: Transform::from_scale(Vec3::splat(0.7)),
            ..Default::default()
        },
    ));

    create_gravity2d_boundaries(&mut commands);
}
