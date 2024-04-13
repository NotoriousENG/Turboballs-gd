use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use godot::engine::{Camera3D, INode3D, Label, MeshInstance3D, Node3D, RandomNumberGenerator};
use godot::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct Turboballs {
    base: Base<Node3D>,
    rms_volume: Arc<Mutex<f32>>,
    _stream: Option<cpal::Stream>,
    player: Option<Gd<Node3D>>,
    mic_label: Option<Gd<Label>>,
    camera: Option<Gd<Camera3D>>,
    ball: Option<Gd<Node3D>>,
    enemy: Option<Gd<Node3D>>,
    t: f32,
    ball_begin_pos: Vector3,
    ball_end_pos: Vector3,
    rng: Gd<RandomNumberGenerator>,
}

const X_RANGE: f32 = 9.0;
const CAM_RANGE: f32 = 12.0;
const BALL_OFFSET: f32 = 3.0;
const PLAYER_BALL_DEST: f32 = 12.0;
const MAX_BALL_HEIGHT: f32 = 4.0;

fn calculate_max_volume(buffer: &[f32]) -> f32 {
    // calculate the max volume of the buffer (clamp 0-1)
    buffer
        .iter()
        .fold(0.0, |max, &x| if x > max { x } else { max })
        .max(0.0)
        .min(1.0)
}

fn lerpf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[godot_api]
impl INode3D for Turboballs {
    fn init(base: Base<Node3D>) -> Self {
        godot_print!("Loaded turboballs!");

        // setup audio input
        let host = cpal::default_host();
        let device = host.default_input_device().unwrap();
        let config = device.default_input_config().unwrap();
        let rms_volume = Arc::new(Mutex::new(0.0));
        let rms_volume_clone = Arc::clone(&rms_volume);
        let stream = match device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let rms_volume = calculate_max_volume(data);
                *rms_volume_clone.lock().unwrap() = rms_volume;
            },
            move |err| {
                godot_print!("an error occurred on the audio stream: {:?}", err);
            },
            None,
        ) {
            Ok(stream) => match stream.play() {
                Ok(_) => {
                    godot_print!("Audio input stream started!");
                    Some(stream)
                }
                Err(e) => {
                    godot_print!("Failed to start audio input stream: {:?}", e);
                    None
                }
            },
            Err(e) => {
                godot_print!("Failed to build audio input stream: {:?}", e);
                None
            }
        };

        Self {
            base,
            rms_volume,
            _stream: stream,
            player: None,
            mic_label: None,
            camera: None,
            ball: None,
            enemy: None,
            t: 0.0,
            ball_begin_pos: Vector3::new(-X_RANGE + BALL_OFFSET, 0.0, 0.0),
            ball_end_pos: Vector3::new(-X_RANGE, 0.0, PLAYER_BALL_DEST),
            rng: RandomNumberGenerator::new_gd(),
        }
    }
    fn ready(&mut self) {
        self.player = Some(self.base().get_node_as::<Node3D>("player"));
        self.mic_label = Some(self.base().get_node_as::<Label>("hud/mic_label"));
        self.camera = Some(self.base().get_node_as::<Camera3D>("camera"));
        self.ball = Some(self.base().get_node_as::<Node3D>("ball"));
        self.enemy = Some(self.base().get_node_as::<Node3D>("enemy"));
    }

    fn process(&mut self, delta: f64) {
        self.t += 0.4 * delta as f32;

        let mic_label = self.mic_label.as_mut().unwrap();
        let rms_volume = *self.rms_volume.lock().unwrap();
        let volume_percent = (rms_volume * 100.0).round();
        mic_label.set_text(GString::from(format!("Mic: {}%", volume_percent)));

        let player = self.player.as_mut().unwrap();
        // set the player's x position based on the rms volume (0 is -9, 1 is 9)
        let x = (rms_volume * X_RANGE * 2.0) - X_RANGE;
        let pos = player.get_position();
        player.set_position(Vector3::new(x, pos.y, pos.z));

        let cam = self.camera.as_mut().unwrap();
        let cam_pos = cam.get_position();
        let cam_x = (rms_volume * CAM_RANGE * 2.0) - CAM_RANGE;
        cam.set_position(Vector3::new(cam_x, cam_pos.y, cam_pos.z));
        // cam look at origin
        cam.look_at(Vector3::new(0.0, 0.0, 0.0));

        let ball = self.ball.as_mut().unwrap();
        let mut ball_pos = ball.get_position();

        ball_pos = Vector3::lerp(self.ball_begin_pos, self.ball_end_pos, self.t);

        // override the y to be max at t = 0.5, 0.1 at t = 0.0 and 1.0
        ball_pos.y = MAX_BALL_HEIGHT * 4.0 * self.t * (1.0 - self.t) + 0.1;

        ball.set_position(ball_pos);

        // if t is 1.0f reset t and swap ball pos
        if self.t >= 1.0 {
            self.t = 0.1;
            self.ball_begin_pos = ball_pos;
            if self.ball_end_pos.z >= PLAYER_BALL_DEST {
                self.ball_end_pos.z = 0.0;
            } else {
                self.ball_end_pos.z = PLAYER_BALL_DEST;
            }

            // set a random x position for ball end pos based on cam max
            // use godot rng
            self.rng.randomize();
            let x = self.rng.randf_range(-CAM_RANGE, CAM_RANGE);
            self.ball_end_pos.x = x;
            let color: Color = Color::from_rgb(
                self.rng.randf_range(0.0, 1.0),
                self.rng.randf_range(0.0, 1.0),
                self.rng.randf_range(0.0, 1.0),
            );
            let mesh = ball.get_node_as::<MeshInstance3D>("Sphere");
            // set the albedo color of the ball
            let mut mat = mesh.get_active_material(0).unwrap();
            mat.set("albedo_color".into(), Variant::from(color));
        }

        // lerp the enemy z pos to always be the ballEndPos x
        let enemy = self.enemy.as_mut().unwrap();
        let enemy_pos = enemy.get_position();
        // lerp the enemy x pos to always be the ballEndPos x
        let enemy_x = lerpf(enemy_pos.x, self.ball_end_pos.x, self.t);
        enemy.set_position(Vector3::new(enemy_x, enemy_pos.y, enemy_pos.z));
    }
}