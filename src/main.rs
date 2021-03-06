use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

fn main() {
    App::build()
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(Inputs::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .add_startup_system(setup.system())
        .add_system_to_stage(
            CoreStage::PreUpdate,
            input_decoder.system().label("input_decoder"),
        )
        .add_system_to_stage(
            CoreStage::PreUpdate,
            handle_inputs.system().after("input_decoder"),
        )
        .add_system(movement.system().label("movement"))
        .add_system(paddle_limiter.system().after("movement"))
        .add_system(ball_limiter.system().after("movement"))
        .add_system(ball_paddle_collider.system().after("movement"))
        .run();
}

//Resources
#[derive(Clone, Copy)]
struct PaddleInputs {
    up: bool,
    down: bool,
}

struct Inputs {
    left: PaddleInputs,
    right: PaddleInputs,
}

impl PaddleInputs {
    fn new(up: bool, down: bool) -> Self {
        PaddleInputs { up, down }
    }
}

impl Default for Inputs {
    fn default() -> Self {
        Inputs {
            left: PaddleInputs::new(false, false),
            right: PaddleInputs::new(false, false),
        }
    }
}

//Components
enum Player {
    Left,
    Right,
}
struct Ball;
struct Paddle(Player);
struct Velocity(Vec2);

//Helpers
fn spawn_paddle(
    shape: &shapes::Rectangle,
    color: Color,
    player: Player,
    transform: Transform,
    commands: &mut Commands,
) {
    commands
        .spawn_bundle(GeometryBuilder::build_as(
            shape,
            ShapeColors::new(color),
            DrawMode::Fill(FillOptions::default()),
            transform,
        ))
        .insert(Paddle(player))
        .insert(Velocity(Vec2::ZERO));
}

//Systems
fn setup(mut commands: Commands) {
    let color_geometry = Color::rgb(0.9, 0.9, 0.9);
    let shape_ball = shapes::Circle {
        radius: 8.0,
        center: Vec2::ZERO,
    };
    let shape_paddle = shapes::Rectangle {
        height: 64.0,
        width: 16.0,
        origin: shapes::RectangleOrigin::Center,
    };

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(GeometryBuilder::build_as(
            &shape_ball,
            ShapeColors::new(color_geometry),
            DrawMode::Fill(FillOptions::default()),
            Transform::default(),
        ))
        .insert(Ball)
        .insert(Velocity(Vec2::new(150.0, 150.0)));

    let mut paddle_left_transform = Transform::default();
    paddle_left_transform.translation.x = -500.0;
    spawn_paddle(
        &shape_paddle,
        color_geometry,
        Player::Left,
        paddle_left_transform,
        &mut commands,
    );

    let mut paddle_right_transform = Transform::default();
    paddle_right_transform.translation.x = 500.0;
    spawn_paddle(
        &shape_paddle,
        color_geometry,
        Player::Right,
        paddle_right_transform,
        &mut commands,
    );
}

fn input_decoder(keys: Res<Input<KeyCode>>, mut inputs: ResMut<Inputs>) {
    inputs.left.up = keys.pressed(KeyCode::W);
    inputs.left.down = keys.pressed(KeyCode::S);
    inputs.right.up = keys.pressed(KeyCode::I);
    inputs.right.down = keys.pressed(KeyCode::K);
}

fn handle_inputs(inputs: Res<Inputs>, mut query: Query<(&Paddle, &mut Velocity)>) {
    for (paddle, mut velocity) in query.iter_mut() {
        let input = match &paddle.0 {
            Player::Left => inputs.left,
            Player::Right => inputs.right,
        };

        velocity.0.y = 0.0;
        if input.up {
            velocity.0.y += 600.0
        };
        if input.down {
            velocity.0.y -= 600.0
        };
    }
}

fn movement(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity)>) {
    let dt = time.delta_seconds();

    for (mut transform, velocity) in query.iter_mut() {
        transform.translation += velocity.0.extend(0.0) * dt;
    }
}

fn paddle_limiter(mut query: Query<&mut Transform, With<Paddle>>) {
    const PADDLE_HALF_HEIGHT: f32 = 32.0;
    const SCREEN_HALF_HEIGHT: f32 = 360.0;

    for mut transform in query.iter_mut() {
        if transform.translation.y > SCREEN_HALF_HEIGHT - PADDLE_HALF_HEIGHT {
            transform.translation.y = SCREEN_HALF_HEIGHT - PADDLE_HALF_HEIGHT;
        }

        if transform.translation.y < -SCREEN_HALF_HEIGHT + PADDLE_HALF_HEIGHT {
            transform.translation.y = -SCREEN_HALF_HEIGHT + PADDLE_HALF_HEIGHT;
        }
    }
}

fn ball_limiter(mut query: Query<(&mut Transform, &mut Velocity), With<Ball>>) {
    const BALL_RADIUS: f32 = 8.0;
    const SCREEN_HALF_HEIGHT: f32 = 360.0;
    const SCREEN_HALF_WIDTH: f32 = 640.0;

    for (mut transform, mut velocity) in query.iter_mut() {
        //Bounce at top
        let mut flip_vert = false;
        if transform.translation.y > SCREEN_HALF_HEIGHT - BALL_RADIUS {
            transform.translation.y = SCREEN_HALF_HEIGHT - BALL_RADIUS;
            flip_vert = true;
        }

        if transform.translation.y < -SCREEN_HALF_HEIGHT + BALL_RADIUS {
            transform.translation.y = -SCREEN_HALF_HEIGHT + BALL_RADIUS;
            flip_vert = true;
        }

        if flip_vert {
            velocity.0.y = -velocity.0.y;
        }

        //Recenter at edges
        let mut ball_out = false;
        if transform.translation.x < -SCREEN_HALF_WIDTH - BALL_RADIUS {
            ball_out = true;
        }

        if transform.translation.x > SCREEN_HALF_WIDTH + BALL_RADIUS {
            ball_out = true;
        }

        if ball_out {
            transform.translation = Vec3::ZERO;
            velocity.0.x = -velocity.0.x;
        }
    }
}

fn ball_paddle_collider(
    mut balls: Query<(&Transform, &mut Velocity), With<Ball>>,
    paddles: Query<&Transform, With<Paddle>>,
) {
    const BALL_DIAMETER: f32 = 8.0;
    const PADDLE_WIDTH: f32 = 16.0;
    const PADDLE_HEIGHT: f32 = 64.0;

    for (ball_transform, mut ball_velocity) in balls.iter_mut() {
        for paddle_transform in paddles.iter() {
            let ball_pos = ball_transform.translation;
            let paddle_pos = paddle_transform.translation;

            if ball_pos.x < paddle_pos.x + PADDLE_WIDTH
                && ball_pos.x + BALL_DIAMETER > paddle_pos.x
                && ball_pos.y > paddle_pos.y - PADDLE_HEIGHT
                && ball_pos.y - BALL_DIAMETER < paddle_pos.y
            {
                ball_velocity.0.x = -ball_velocity.0.x;
            }
        }
    }
}
