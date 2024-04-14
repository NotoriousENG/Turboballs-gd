use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use godot::engine::utilities::sign;
use godot::engine::{
    Camera3D, Control, DisplayServer, INode3D, Label, MeshInstance3D, Node3D, Os,
    RandomNumberGenerator,
};
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
    score_label: Option<Gd<Label>>,
    score: i32,
    high_score_label: Option<Gd<Label>>,
    high_score: i32,
    is_playing: bool,
    playing_control_ui: Option<Gd<Control>>,
    start_control_ui: Option<Gd<Control>>,
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
        // setup audio input
        let host = cpal::default_host();
        let device = host.default_input_device().unwrap();
        #[cfg(target_os = "android")]
        {
            // let p = Os::singleton().request_permission("android.permission.RECORD_AUDIO".into());
            let p = false;
            let permissions = Os::singleton().get_granted_permissions();
            godot_print!("Permissions: {:?}", permissions);
            if !p {
                godot_print!("Failed to get audio permission");
                return Self {
                    base,
                    rms_volume: Arc::new(Mutex::new(0.0)),
                    _stream: None,
                    player: None,
                    mic_label: None,
                    camera: None,
                    ball: None,
                    enemy: None,
                    t: 0.0,
                    ball_begin_pos: Vector3::new(-X_RANGE + BALL_OFFSET, 0.0, 0.0),
                    ball_end_pos: Vector3::new(-X_RANGE, 0.0, PLAYER_BALL_DEST),
                    rng: RandomNumberGenerator::new_gd(),
                    score_label: None,
                    score: 0,
                    high_score_label: None,
                    high_score: 0,
                    is_playing: false,
                    playing_control_ui: None,
                    start_control_ui: None,
                };
            }
        }

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
            score_label: None,
            score: 0,
            high_score_label: None,
            high_score: 0,
            is_playing: false,
            playing_control_ui: None,
            start_control_ui: None,
        }
    }
    fn ready(&mut self) {
        self.player = Some(self.base().get_node_as::<Node3D>("player"));
        self.mic_label = Some(self.base().get_node_as::<Label>("hud/playing/mic_label"));
        self.camera = Some(self.base().get_node_as::<Camera3D>("camera"));
        self.ball = Some(self.base().get_node_as::<Node3D>("ball"));
        self.enemy = Some(self.base().get_node_as::<Node3D>("enemy"));
        self.score_label = Some(self.base().get_node_as::<Label>("hud/playing/score_label"));
        self.high_score_label = Some(
            self.base()
                .get_node_as::<Label>("hud/playing/hi_score_label"),
        );
        self.t = 0.0;
        self.is_playing = false;
        self.playing_control_ui = Some(self.base().get_node_as::<Control>("hud/playing"));
        self.start_control_ui = Some(self.base().get_node_as::<Control>("hud/start"));
        self.playing_control_ui.as_mut().unwrap().hide();
        self.start_control_ui.as_mut().unwrap().show();
    }

    fn process(&mut self, delta: f64) {
        #[cfg(target_os = "android")]
        {
            let mut s = -1.0;
            if Input::singleton().is_action_pressed("tb_start".into()) {
                s = 1.0;
            }
            let mut rms_volume = self.rms_volume.lock().unwrap();
            *rms_volume = (*rms_volume + (delta as f32) * s).clamp(0.0, 1.0);
        }
        #[cfg(not(target_os = "android"))]
        {
            if Input::singleton().is_action_just_pressed("toggle_fullscreen".into()) {
                let fullscreen = godot::engine::display_server::WindowMode::FULLSCREEN;
                let is_fullscreen = DisplayServer::singleton().window_get_mode() == fullscreen;
                if is_fullscreen {
                    DisplayServer::singleton()
                        .window_set_mode(godot::engine::display_server::WindowMode::WINDOWED);
                    // set the window size to 800x600
                    DisplayServer::singleton().window_set_size(Vector2i::new(800, 600));
                } else {
                    DisplayServer::singleton().window_set_mode(fullscreen);
                }
            }
        }
        if !self.is_playing {
            // if enter is pressed reset the game
            if Input::singleton().is_action_just_pressed("tb_start".into()) {
                self.score = 0;
                let score_label = self.score_label.as_mut().unwrap();
                score_label.set_text(GString::from(format!("Score: {}", self.score)));
                self.is_playing = true;
                self.t = 1.01;
                self.start_control_ui.as_mut().unwrap().hide();
                self.playing_control_ui.as_mut().unwrap().show();
            }
            return;
        }
        self.t += 0.4 * delta as f32;

        let mic_label = self.mic_label.as_mut().unwrap();
        let rms_volume = *self.rms_volume.lock().unwrap();
        let volume_percent = (rms_volume * 100.0).round();
        mic_label.set_text(GString::from(format!("Mic: {}%", volume_percent)));

        let player = self.player.as_mut().unwrap();
        // set the player's x position based on the rms volume (0 is -9, 1 is 9)
        let mut player_pos = player.get_position();
        player_pos.x = (rms_volume * X_RANGE * 2.0) - X_RANGE;
        player.set_position(player_pos);

        let cam = self.camera.as_mut().unwrap();
        let mut cam_pos = cam.get_position();
        cam_pos.x = (rms_volume * CAM_RANGE * 2.0) - CAM_RANGE;
        cam.set_position(cam_pos);
        // cam look at origin
        cam.look_at(Vector3::new(0.0, 0.0, 0.0));

        let ball = self.ball.as_mut().unwrap();
        let mut ball_pos = ball.get_position();

        if self.ball_end_pos.z == PLAYER_BALL_DEST {
            if Vector3::distance_to(ball_pos, player_pos) < 3.5 {
                self.score += 1;
                self.t = 1.1;
                let score_label = self.score_label.as_mut().unwrap();
                score_label.set_text(GString::from(format!("Score: {}", self.score)));
                if self.score > self.high_score {
                    self.high_score = self.score;
                    let high_score_label = self.high_score_label.as_mut().unwrap();
                    high_score_label
                        .set_text(GString::from(format!("High Score: {}", self.high_score)));
                }

                // if it is 0.1 away from dest
            } else if PLAYER_BALL_DEST - ball_pos.z < 0.1 {
                self.t = 1.1;
                self.is_playing = false;
                self.playing_control_ui.as_mut().unwrap().hide();
                self.start_control_ui.as_mut().unwrap().show();
            }
        }

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
            let x = self.rng.randf_range(-X_RANGE, X_RANGE);
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
