use bevy::math::Vec3Swizzles;
//use bevy::time::FixedTimestep;
use bevy::{prelude::*, window::PresentMode};

const TICK_RATE: f64 = 1.0 / 20.0;
const VISUAL_SLOWDOWN: f64 = 1.0;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .insert_resource(Msaa::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_editor_pls::EditorPlugin::new())
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_rope)
        .add_startup_system(setup_translation)
        .add_startup_system(setup_rotational)
        .add_systems((
            symplectic_euler,
            spring_impulse.before(symplectic_euler),
            gravity.before(symplectic_euler),
        ))
        .register_type::<Impulse>()
        .register_type::<Gravity>()
        .register_type::<Inertia>()
        .register_type::<Velocity>()
        .register_type::<SpringSettings>()
        .run();
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0.0, 300.0, 0.0),
        ..default()
    });
}

#[derive(Debug, Copy, Clone, Component)]
pub struct Spring {
    pub containing: Entity,
}

#[derive(Default, Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct SpringSettings(springy::Spring);

#[derive(Default, Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Velocity {
    pub linear: Vec2,
    pub angular: f32,
}

#[derive(Default, Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Impulse {
    pub linear: Vec2,
    pub angular: f32,
}

#[derive(Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Inertia {
    pub linear: f32,
    pub angular: f32,
}

impl Default for Inertia {
    fn default() -> Self {
        Self {
            linear: 1.0,
            angular: 1.0,
        }
    }
}

impl Inertia {
    pub const INFINITY: Self = Inertia {
        linear: f32::INFINITY,
        angular: f32::INFINITY,
    };

    pub fn inverse_linear(&self) -> f32 {
        if self.linear.is_normal() {
            1.0 / self.linear
        } else {
            0.0
        }
    }

    pub fn inverse_angular(&self) -> f32 {
        if self.angular.is_normal() {
            1.0 / self.angular
        } else {
            0.0
        }
    }
}

#[derive(Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct Gravity(pub Vec2);

impl Default for Gravity {
    fn default() -> Self {
        Self(Vec2::new(0.0, -9.817))
    }
}

/// Basic symplectic euler integration of the impulse/velocity/position.
pub fn symplectic_euler(
    time: Res<Time>,
    mut to_integrate: Query<(&mut Transform, &mut Velocity, &mut Impulse, &Inertia)>,
) {
    if time.delta_seconds() == 0.0 {
        return;
    }

    for (mut position, mut velocity, mut impulse, inertia) in &mut to_integrate {
        velocity.linear += impulse.linear * inertia.inverse_linear();
        velocity.angular += impulse.angular * inertia.inverse_angular();

        position.translation += velocity.linear.extend(0.0) * TICK_RATE as f32;
        //position.rotation += velocity.angular.extend(0.0) * TICK_RATE as f32;

        impulse.linear = Vec2::ZERO;
        impulse.angular = 0.0;
    }
}

pub fn gravity(time: Res<Time>, mut to_apply: Query<(&mut Impulse, &Gravity)>) {
    if time.delta_seconds() == 0.0 {
        return;
    }

    for (mut impulse, gravity) in &mut to_apply {
        impulse.linear += gravity.0;
    }
}

#[derive(Default, Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct PreviousUnitVector(Option<Vec2>);

pub fn spring_impulse(
    time: Res<Time>,
    mut impulses: Query<&mut Impulse>,
    mut springs: Query<(
        Entity,
        &GlobalTransform,
        &Velocity,
        &Inertia,
        &SpringSettings,
        &Spring,
        &mut PreviousUnitVector,
    )>,
    particle: Query<(&GlobalTransform, &Velocity, &Inertia)>,
) {
    if time.delta_seconds() == 0.0 {
        return;
    }

    let timestep = TICK_RATE as f32;

    for (
        spring_entity,
        spring_transform,
        spring_velocity,
        spring_mass,
        spring_settings,
        spring,
        mut previous_unit_vector,
    ) in &mut springs
    {
        let particle_entity = spring.containing;
        let (particle_transform, particle_velocity, particle_mass) =
            particle.get(particle_entity).unwrap();

        if particle_entity == spring_entity {
            continue;
        }
        let particle_a = springy::TranslationParticle2 {
            mass: spring_mass.linear,
            translation: spring_transform.translation().xy(),
            velocity: spring_velocity.linear,
        };
        let particle_b = springy::TranslationParticle2 {
            mass: particle_mass.linear,
            translation: particle_transform.translation().xy(),
            velocity: particle_velocity.linear,
        };

        let instant = particle_a.instant(&particle_b);
        let impulse = spring_settings.0.impulse(timestep, instant);

        let [mut spring_impulse, mut particle_impulse] = impulses
            .get_many_mut([spring_entity, particle_entity])
            .unwrap();

        spring_impulse.linear += impulse;
        particle_impulse.linear -= impulse;
    }
}

pub fn setup_rope(mut commands: Commands) {
    let size = 20.0;
    let sprite = Sprite {
        color: Color::BLUE,
        flip_x: false,
        flip_y: false,
        custom_size: Some(Vec2::new(size, size)),
        rect: None,
        anchor: Default::default(),
    };

    let slot = Sprite {
        color: Color::RED,
        flip_x: false,
        flip_y: false,
        custom_size: Some(Vec2::new(size / 4.0, size / 4.0)),
        rect: None,
        anchor: Default::default(),
    };

    let cube_3 = commands
        .spawn(SpriteBundle {
            sprite: sprite.clone(),
            ..default()
        })
        .insert((
            Velocity::default(),
            Impulse::default(),
            Inertia::default(),
            Gravity::default(),
            PreviousUnitVector::default(),
        ))
        .insert(Name::new("Cube 3"))
        .id();

    let cube_2 = commands
        .spawn(SpriteBundle {
            sprite: sprite.clone(),
            ..default()
        })
        .insert((
            Velocity::default(),
            Impulse::default(),
            Inertia::default(),
            Gravity::default(),
            PreviousUnitVector::default(),
        ))
        .insert(Name::new("Cube 2"))
        .insert(Spring { containing: cube_3 })
        .insert(SpringSettings(springy::Spring {
            strength: 0.05,
            damp_ratio: 1.0,
        }))
        .id();

    let cube_1 = commands
        .spawn(SpriteBundle {
            sprite: sprite.clone(),
            ..default()
        })
        .insert(TransformBundle::from(Transform::from_xyz(50.0, 50.0, 0.0)))
        .insert((
            Velocity::default(),
            Impulse::default(),
            Inertia::default(),
            Gravity::default(),
            PreviousUnitVector::default(),
        ))
        .insert(Spring { containing: cube_2 })
        .insert(SpringSettings(springy::Spring {
            strength: 0.05,
            damp_ratio: 1.0,
        }))
        .insert(Name::new("Cube 1"))
        .id();

    let cube_slot = commands
        .spawn(SpriteBundle {
            sprite: slot.clone(),
            ..default()
        })
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 300.0, 0.0)))
        .insert(Spring { containing: cube_1 })
        .insert(SpringSettings(springy::Spring {
            strength: 0.05,
            damp_ratio: 1.0,
        }))
        .insert((
            Velocity::default(),
            Impulse::default(),
            Inertia::INFINITY,
            PreviousUnitVector::default(),
        ))
        .insert(Name::new("Cube Slot"));
}

pub fn setup_translation(mut commands: Commands) {
    let size = 20.0;
    let sprite = Sprite {
        color: Color::BLUE,
        flip_x: false,
        flip_y: false,
        custom_size: Some(Vec2::new(size, size)),
        rect: None,
        anchor: Default::default(),
    };

    let slot = Sprite {
        color: Color::RED,
        flip_x: false,
        flip_y: false,
        custom_size: Some(Vec2::new(size / 4.0, size / 4.0)),
        rect: None,
        anchor: Default::default(),
    };

    let iterations = 500;
    let height = 500.0;
    for damped in 0..iterations {
        let size = height / iterations as f32;
        let damped_sprite = Sprite {
            color: Color::YELLOW,
            flip_x: false,
            flip_y: false,
            custom_size: Some(Vec2::new(5.0, size)),
            rect: None,
            anchor: Default::default(),
        };

        let height = damped as f32 * size;
        let damped_cube = commands
            .spawn(SpriteBundle {
                sprite: damped_sprite.clone(),
                ..default()
            })
            .insert(TransformBundle::from(Transform::from_xyz(
                300.0, height, 0.0,
            )))
            .insert((
                Velocity::default(),
                Impulse::default(),
                Inertia::default(),
                PreviousUnitVector::default(),
            ))
            .insert(Name::new("Critical"))
            .id();

        let critical_slot = commands
            .spawn(SpriteBundle {
                sprite: slot.clone(),
                ..default()
            })
            .insert(TransformBundle::from(Transform::from_xyz(
                100.0, height, 0.0,
            )))
            .insert(Spring {
                containing: damped_cube,
            })
            .insert(SpringSettings(springy::Spring {
                strength: 0.05,
                damp_ratio: damped as f32 / iterations as f32,
            }))
            .insert((
                Velocity::default(),
                Impulse::default(),
                Inertia::INFINITY,
                PreviousUnitVector::default(),
            ))
            .insert(Name::new("Critical Slot"));
    }
}

pub fn setup_rotational(mut commands: Commands) {
    let size = 20.0;
    let sprite = Sprite {
        color: Color::BLUE,
        flip_x: false,
        flip_y: false,
        custom_size: Some(Vec2::new(size, size)),
        rect: None,
        anchor: Default::default(),
    };

    let slot = Sprite {
        color: Color::RED,
        flip_x: false,
        flip_y: false,
        custom_size: Some(Vec2::new(size / 4.0, size / 4.0)),
        rect: None,
        anchor: Default::default(),
    };

    let iterations = 500;
    let height = 500.0;
    for damped in 0..iterations {
        let size = height / iterations as f32;
        let damped_sprite = Sprite {
            color: Color::YELLOW,
            flip_x: false,
            flip_y: false,
            custom_size: Some(Vec2::new(5.0, size)),
            rect: None,
            anchor: Default::default(),
        };

        let height = damped as f32 * size;
        let damped_cube = commands
            .spawn(SpriteBundle {
                sprite: damped_sprite.clone(),
                ..default()
            })
            .insert(TransformBundle::from(Transform::from_xyz(
                -300.0, height, 0.0,
            )))
            .insert((
                Velocity::default(),
                Impulse::default(),
                Inertia::default(),
                PreviousUnitVector::default(),
            ))
            .insert(Name::new("Critical"))
            .id();

        let critical_slot = commands
            .spawn(SpriteBundle {
                sprite: slot.clone(),
                ..default()
            })
            .insert(TransformBundle::from(Transform::from_xyz(
                -100.0, height, 0.0,
            )))
            .insert(Spring {
                containing: damped_cube,
            })
            .insert(SpringSettings(springy::Spring {
                strength: 0.05,
                damp_ratio: damped as f32 / iterations as f32,
            }))
            .insert((
                Velocity::default(),
                Impulse::default(),
                Inertia::INFINITY,
                PreviousUnitVector::default(),
            ))
            .insert(Name::new("Critical Slot"));
    }
}
