mod keyboard;
mod keyboard_pipeline;

use keyboard::PianoKeyboard;

mod notes;
mod notes_pipeline;

use notes::Notes;

use super::{Scene, SceneEvent, SceneType};
use lib_midi::MidiNote;

use crate::{
    main_state::MainState,
    rectangle_pipeline::{RectangleInstance, RectanglePipeline},
    target::Target,
    time_manager::Timer,
    wgpu_jumpstart::Color,
};

use winit::event::WindowEvent;

pub struct PlayingScene {
    piano_keyboard: PianoKeyboard,
    notes: Notes,
    player: Player,
    rectangle_pipeline: RectanglePipeline,

    text_toast: Option<Toast>,

    keyboard_on: bool,  //editted for test
}

impl PlayingScene {
    pub fn new(target: &mut Target) -> Self {
        let piano_keyboard = PianoKeyboard::new(target);

        let mut notes = Notes::new(target, &piano_keyboard.all_keys);

        let player = Player::new(&mut target.state);
        notes.update(target, player.time);

        Self {
            piano_keyboard,
            notes,
            player,
            rectangle_pipeline: RectanglePipeline::new(&target.gpu, &target.transform_uniform),

            text_toast: None,
            keyboard_on: true  //editted for test
        }
    }

    fn speed_toast(&mut self, target: &mut Target) {
        let s = format!(
            "Speed: {}",
            (target.state.config.speed_multiplier * 100.0).round() / 100.0
        );

        self.text_toast = Some(Toast::new(move |target| {
            let text = vec![wgpu_glyph::Text::new(&s)
                .with_color([1.0, 1.0, 1.0, 1.0])
                .with_scale(20.0)];

            target.text_renderer.queue_text(wgpu_glyph::Section {
                text,
                screen_position: (0.0, 20.0),
                layout: wgpu_glyph::Layout::Wrap {
                    line_breaker: Default::default(),
                    h_align: wgpu_glyph::HorizontalAlign::Left,
                    v_align: wgpu_glyph::VerticalAlign::Top,
                },
                ..Default::default()
            });
        }));
    }

    fn offset_toast(&mut self, target: &mut Target) {
        let s = format!(
            "Offset: {}",
            (target.state.config.playback_offset * 100.0).round() / 100.0
        );

        self.text_toast = Some(Toast::new(move |target| {
            let text = vec![wgpu_glyph::Text::new(&s)
                .with_color([1.0, 1.0, 1.0, 1.0])
                .with_scale(20.0)];

            target.text_renderer.queue_text(wgpu_glyph::Section {
                text,
                screen_position: (0.0, 20.0),
                layout: wgpu_glyph::Layout::Wrap {
                    line_breaker: Default::default(),
                    h_align: wgpu_glyph::HorizontalAlign::Left,
                    v_align: wgpu_glyph::VerticalAlign::Top,
                },
                ..Default::default()
            });
        }));
    }

    #[cfg(feature = "record")]
    pub fn playback_progress(&self) -> f32 {
        self.player.percentage * 100.0
    }
}

impl Scene for PlayingScene {
    fn done(mut self: Box<Self>, target: &mut Target) {
        self.player.clear(&mut target.state);
    }

    fn scene_type(&self) -> SceneType {
        SceneType::Playing
    }
    fn start(&mut self) {
        self.player.start();
    }
    fn resize(&mut self, target: &mut Target) {
        self.piano_keyboard.resize(target);
        self.notes.resize(target, &self.piano_keyboard.all_keys);
    }
    fn update(&mut self, target: &mut Target) -> SceneEvent {
        let (window_w, window_h) = {
            let winit::dpi::LogicalSize { width, height } = target.window.state.logical_size;
            (width, height)
        };

        let notes_on = self.player.update(&mut target.state);

        let size_x = window_w * self.player.percentage;

        self.rectangle_pipeline.update_instance_buffer(
            &mut target.gpu.encoder,
            &target.gpu.device,
            vec![RectangleInstance {
                position: [0.0, 0.0],
                size: [size_x, 5.0],
                color: Color::from_rgba8(56, 145, 255, 1.0).into_linear_rgba(),
            }],
        );

        let pos = &target.window.state.cursor_logical_position;
        if pos.y < 20.0
            && target
                .window
                .state
                .mouse_is_pressed(winit::event::MouseButton::Left)
        {
            let x = pos.x;
            let p = x / window_w;
            log::debug!("Progressbar Clicked: x:{},p:{}", x, p);
            self.player.set_percentage_time(&mut target.state, p);

            if !self.player.rewind_controler.is_rewinding() {
                self.player.start_rewind(RewindControler::Mouse {
                    was_paused: self.player.timer.paused,
                });
            }
        } else if let RewindControler::Mouse { .. } = self.player.rewind_controler {
            self.player.stop_rewind();
        }

        self.piano_keyboard.update_notes_state(target, notes_on);
        /*if !self.keyboard_on {
            self.notes.update(
                target,
                self.player.time + target.state.config.playback_offset -0.1,       // 판정 위치 바뀜 .. 판정 위치가 오락가락하는 이유는 time을 round하는 과정에서 발생하는 것이 아닐까
            );
        } else {
            self.notes.update(
                target,
                self.player.time + target.state.config.playback_offset + window_h / 5.0,
            );
        }*/
        self.notes.update(
            target,
            self.player.time + target.state.config.playback_offset,       // 판정 위치 바뀜 .. 판정 위치가 오락가락하는 이유는 time을 round하는 과정에서 발생하는 것이 아닐까(판정 위치를 결정하는 것이 아니라 노트의 위치를 결정하는 듯)
        );

        // Toasts
        {
            if let Some(mut toast) = self.text_toast.take() {
                self.text_toast = if toast.draw(target) {
                    Some(toast)
                } else {
                    None
                };
            }
        }

        //log::warn!("percentage: {:?}", self.player.percentage);

        if self.player.percentage >= 1.0 {
            //log::warn!("End");
            return SceneEvent::GoBack
        } else {
            SceneEvent::None
        }
    }
    fn render(&mut self, target: &mut Target, view: &wgpu::TextureView) {
        let transform_uniform = &target.transform_uniform;
        let encoder = &mut target.gpu.encoder;
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            self.notes.render(&transform_uniform, &mut render_pass);


            //editted for test - 키보드 색깔 입히는 부분(아마도)



            if !self.keyboard_on {
                self.piano_keyboard
                    .render(&transform_uniform, &mut render_pass);     
            }


            
            self.rectangle_pipeline
                .render(&target.transform_uniform, &mut render_pass)
        }
    }
    fn window_event(&mut self, target: &mut Target, event: &WindowEvent) -> SceneEvent {
        match &event {
            winit::event::WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                Some(winit::event::VirtualKeyCode::Escape) => {
                    if let winit::event::ElementState::Released = input.state {
                        return SceneEvent::GoBack;
                    }
                }
                Some(winit::event::VirtualKeyCode::Space) => {
                    if let winit::event::ElementState::Released = input.state {
                        self.player.pause_resume(&mut target.state);
                    }
                }
                Some(winit::event::VirtualKeyCode::Left) => {
                    if let winit::event::ElementState::Pressed = input.state {
                        let speed = if target.window.state.modifers_state.shift() {
                            -0.0001 * 50.0
                        } else {
                            -0.0001
                        };

                        if !self.player.rewind_controler.is_rewinding() {
                            self.player.start_rewind(RewindControler::Keyboard {
                                speed,
                                was_paused: self.player.timer.paused,
                            });
                        }
                    } else {
                        self.player.stop_rewind();
                    }
                }
                Some(winit::event::VirtualKeyCode::Right) => {
                    if let winit::event::ElementState::Pressed = input.state {
                        let speed = if target.window.state.modifers_state.shift() {
                            0.0001 * 50.0
                        } else {
                            0.0001
                        };

                        if !self.player.rewind_controler.is_rewinding() {
                            self.player.start_rewind(RewindControler::Keyboard {
                                speed,
                                was_paused: self.player.timer.paused,
                            });
                        }
                    } else {
                        self.player.stop_rewind();
                    }
                }
                Some(winit::event::VirtualKeyCode::Up) => {
                    if let winit::event::ElementState::Released = input.state {
                        if target.window.state.modifers_state.shift() {
                            target.state.config.speed_multiplier += 0.5;
                        } else {
                            target.state.config.speed_multiplier += 0.1;
                        }

                        self.player
                            .set_percentage_time(&mut target.state, self.player.percentage);

                        self.speed_toast(target);
                    }
                }
                Some(winit::event::VirtualKeyCode::Down) => {
                    if let winit::event::ElementState::Released = input.state {
                        let new = if target.window.state.modifers_state.shift() {
                            target.state.config.speed_multiplier - 0.5
                        } else {
                            target.state.config.speed_multiplier - 0.1
                        };

                        if new > 0.0 {
                            target.state.config.speed_multiplier = new;
                            self.player
                                .set_percentage_time(&mut target.state, self.player.percentage);
                        }

                        self.speed_toast(target);
                    }
                }
                Some(winit::event::VirtualKeyCode::Minus) => {
                    if let winit::event::ElementState::Released = input.state {
                        if target.window.state.modifers_state.shift() {
                            target.state.config.playback_offset -= 0.1;
                        } else {
                            target.state.config.playback_offset -= 5.0;
                        }

                        self.offset_toast(target);
                    }
                }
                Some(winit::event::VirtualKeyCode::Plus)
                | Some(winit::event::VirtualKeyCode::Equals) => {
                    if let winit::event::ElementState::Released = input.state {
                        if target.window.state.modifers_state.shift() {
                            target.state.config.playback_offset += 0.1;
                        } else {
                            target.state.config.playback_offset += 0.01;
                        }

                        self.offset_toast(target);
                    }
                }
                Some(winit::event::VirtualKeyCode::Tab) => {
                    if let winit::event::ElementState::Released = input.state {
                        if target.window.state.modifers_state.shift() {

                        } else {
                            if self.keyboard_on {
                                self.keyboard_on = false;
                            } else {
                                self.keyboard_on = true;
                            }
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }

        SceneEvent::None
    }
}

use std::collections::HashMap;

struct Player {
    midi_first_note_start: f32,
    midi_last_note_end: f32,
    active_notes: HashMap<usize, MidiNote>,
    timer: Timer,
    percentage: f32,
    time: f32,

    rewind_controler: RewindControler,
    #[cfg(feature = "play_along")]
    play_along_controler: Option<PlayAlongControler>,
}

impl Player {
    fn new(main_state: &mut MainState) -> Self {
        let midi_file = main_state.midi_file.as_ref().unwrap();

        let midi_first_note_start = if let Some(note) = midi_file.merged_track.notes.first() {
            note.start
        } else {
            0.0
        };
        let midi_last_note_end = if let Some(note) = midi_file.merged_track.notes.last() {
            note.start + note.duration
        } else {
            0.0
        };

        #[cfg(feature = "play_along")]
        let play_along_controler = if main_state.config.play_along {
            PlayAlongControler::new()
        } else {
            None
        };

        let mut player = Self {
            midi_first_note_start,
            midi_last_note_end,
            active_notes: HashMap::new(),
            timer: Timer::new(),
            percentage: 0.0,
            time: 0.0,

            rewind_controler: RewindControler::None,
            #[cfg(feature = "play_along")]
            play_along_controler,
        };
        player.update(main_state);

        player
    }
    fn start(&mut self) {
        self.timer.start();
    }

    fn update(&mut self, main_state: &mut MainState) -> [(bool, usize); 88] {
        if let RewindControler::Keyboard { speed, .. } = self.rewind_controler {
            let p = self.percentage + speed;
            self.set_percentage_time(main_state, p);
        }

        self.timer.update();
        let raw_time = self.timer.get_elapsed() / 1000.0 * main_state.config.speed_multiplier;
        self.percentage = raw_time / (self.midi_last_note_end + 3.0);
        self.time = raw_time + self.midi_first_note_start - 3.0;

        let mut notes_state: [(bool, usize); 88] = [(false, 0); 88];

        #[cfg(feature = "play_along")]
        if let Some(controler) = &mut self.play_along_controler {
            controler.update(main_state, &mut notes_state, &mut self.timer);
        }

        if self.timer.paused {
            return notes_state;
        };

        let filtered: Vec<&lib_midi::MidiNote> = main_state
            .midi_file
            .as_ref()
            .unwrap()
            .merged_track
            .notes
            .iter()
            .filter(|n| n.start <= self.time && n.start + n.duration + 0.5 > self.time)
            .collect();

        let output_manager = &mut main_state.output_manager;

        for n in filtered {
            use std::collections::hash_map::Entry;

            if n.start + n.duration >= self.time {
                if n.note >= 21 + 15 && n.note <= 108 - 12 - 3 && n.ch != 9 && n.ch != 8 {
                    notes_state[n.note as usize - 21 - 15] = (true, n.track_id);
                }

                if let Entry::Vacant(_e) = self.active_notes.entry(n.id) {
                    self.active_notes.insert(n.id, n.clone());

                    #[cfg(feature = "play_along")]
                    if let Some(controler) = &mut self.play_along_controler {
                        if n.note >= 21 + 15 && n.note <= 108 - 12 - 3 && n.ch != 9 && n.ch != 8 {
                            controler.require_note(&mut self.timer, &n);
                            //log::warn!("required_notes: {:?}", controler.required_notes.lock().unwrap().len());
                        } else {
                            //if controler.waiting_for_note == false {
                                //output_manager.note_on(n.ch, n.note, n.vel);
                            //}
                            /*let mut note_temp = [{0}; 3].to_vec();
                            note_temp.push(n.ch);
                            note_temp.push(n.note);
                            note_temp.push(n.vel);
                            controler.outrange_note_list.push(note_temp);   범위 밖 노트가 동시에 눌려야 하는 것이 여러 개인 경우... 아직 미완성(아마도 타입 에러 해결해야 함)*/
                            controler.outrange_note_list.push(n.ch);
                            controler.outrange_note_list.push(n.note);
                            controler.outrange_note_list.push(n.vel);
                            controler.outrange_note = true; 
                            //output_manager.note_on(n.ch, n.note, n.vel);  원래 있던 부분
                        }
                    } else {
                        output_manager.note_on(n.ch, n.note, n.vel);
                    }
                    

                    #[cfg(not(feature = "play_along"))]
                    output_manager.note_on(n.ch, n.note, n.vel);
                }
            } else if let Entry::Occupied(_e) = self.active_notes.entry(n.id) {  

                if !main_state.config.play_along {
                    output_manager.note_off(n.ch, n.note);
                }                                                                 
            }
        }

        notes_state
    }

    fn pause_resume(&mut self, main_state: &mut MainState) {
        self.clear(main_state);
        self.timer.pause_resume();
    }

    fn start_rewind(&mut self, controler: RewindControler) {
        self.timer.pause();
        self.rewind_controler = controler;
    }
    fn stop_rewind(&mut self) {
        let controler = std::mem::replace(&mut self.rewind_controler, RewindControler::None);

        let was_paused = match controler {
            RewindControler::Keyboard { was_paused, .. } => was_paused,
            RewindControler::Mouse { was_paused } => was_paused,
            RewindControler::None => return,
        };

        if !was_paused {
            self.timer.resume();
        }
    }

    fn set_time(&mut self, main_state: &mut MainState, time: f32) {
        self.timer.set_time(time * 1000.0);
        self.clear(main_state);
    }

    fn set_percentage_time(&mut self, main_state: &mut MainState, p: f32) {
        self.set_time(
            main_state,
            p * (self.midi_last_note_end + 3.0) / main_state.config.speed_multiplier,
        );
    }

    fn clear(&mut self, main_state: &mut MainState) {
        for (_id, n) in self.active_notes.iter() {
            main_state.output_manager.note_off(n.ch, n.note);
        }
        self.active_notes.clear();

        #[cfg(feature = "play_along")]
        if let Some(controler) = &mut self.play_along_controler {
            controler.clear();
        }
    }
}

enum RewindControler {
    Keyboard { speed: f32, was_paused: bool },
    Mouse { was_paused: bool },
    None,
}
impl RewindControler {
    fn is_rewinding(&self) -> bool {
        match self {
            RewindControler::None => false,
            _ => true,
        }
    }
}

#[cfg(feature = "play_along")]
use std::sync::{mpsc, Arc, Mutex};
//use std::{thread, time};    //editted for test

#[cfg(feature = "play_along")]
struct PlayAlongControler {
    _midi_in_conn: midir::MidiInputConnection<()>,
    midi_in_rec: mpsc::Receiver<(bool, u8, u8)>,

    input_pressed_keys: [bool; 88],
    required_notes: Arc<Mutex<HashMap<u8, MidiNote>>>,
    waiting_for_note: bool,
    outrange_note: bool,
    outrange_note_list: Vec<u8>,  //범위 밖 노트 한 개
    //outrange_note_list: Vec<Vec<u8>>,  //범위 밖 노트 여러 개
}

#[cfg(feature = "play_along")]
impl PlayAlongControler {
    fn new() -> Option<Self> {
        let input_pressed_keys = [false; 88];
        let required_notes = Arc::new(Mutex::new(HashMap::new()));

        let (tx, midi_in_rec) = mpsc::channel();

        let _midi_in_conn = {
            let midi_in = midir::MidiInput::new("Neothesia-in").unwrap();
            let in_ports = midi_in.ports();

            use std::io::{stdin, stdout, Write};

            let in_port = match in_ports.len() {
                0 => return None,
                1 => {
                    println!(
                        "Choosing the only available input port: {}",
                        midi_in.port_name(&in_ports[0]).unwrap()
                    );
                    &in_ports[0]
                }
                _ => {
                    //println!("\nAvailable input ports:");                             //cargo run --features play_along일떄 (0,1) 묻지 말고 키보드를 input으로 하도록 하지만, 딱 우리 상황에만 맞춘 상태
                    for (i, p) in in_ports.iter().enumerate() {
                        println!("{}: {}", i, midi_in.port_name(p).unwrap());
                    }
                    //print!("Please select input port: ");
                    stdout().flush().unwrap();
                    /*let mut input = String::new();
                    stdin().read_line(&mut input).unwrap();
                    in_ports
                        .get(input.trim().parse::<usize>().unwrap())
                        .ok_or("invalid input port selected")
                        .unwrap()*/
                    in_ports
                        .get("1".trim().parse::<usize>().unwrap())
                        .ok_or("invalid input port selected")
                        .unwrap()
                }
            };

            let required_notes = required_notes.clone();

            midi_in
                .connect(
                    in_port,
                    "neothesia-read-input",
                    move |_, message, _| {
                        if message.len() == 3 {
                            let note = message[1];
                            if note >= 21 && note <= 108  {
                                if message[0] == 128 || message[2] == 0 {
                                    tx.send((false, message[1], message[2])).unwrap();
                                } else if message[0] == 144 {
                                    required_notes.lock().unwrap().remove(&note);
                                    tx.send((true, message[1], message[2])).unwrap();
                                }
                            }
                        }
                    },
                    (),
                )
                .unwrap()
        };

        Some(Self {
            _midi_in_conn,
            midi_in_rec,

            input_pressed_keys,
            required_notes,
            waiting_for_note: false,
            outrange_note: false,
            outrange_note_list: [{0}; 3].to_vec(),  //범위 밖 노트 한개
            //outrange_note_list: [[{0}; 3].to_vec(); 1].to_vec(), 범위 밖 노트 여러 개
        })
    }

    fn update(
        &mut self,
        main_state: &mut MainState,
        notes_state: &mut [(bool, usize); 88],
        timer: &mut Timer,
    ) {
        for (id, is) in self.input_pressed_keys.iter().enumerate() {
            notes_state[id] = (*is, 0);
        }

        if let Ok(event) = self.midi_in_rec.try_recv() {
            if event.0 {
                self.input_pressed_keys[event.1 as usize - 36] = true;
                main_state.output_manager.note_on(0, event.1, event.2);
                if self.required_notes.lock().unwrap().len() == 0 && self.outrange_note == true {  //outrnage_note는 범위 밖 노트가 존재하는지 판별
                    /*let mut counter = 0;
                    self.outrange_note_list.pop();
                    for OutRangeNote in self.outrange_note_list {  //outrange_note_list는 범위 밖 노트의 ch, key, vel 순으로 저장한 벡터 속 벡터 타입으로, required_notes가 empty이면, 즉 동시에 눌려야 하는 노트가 다 눌리면 범위 밖 노트의 소리를 내기 위함
                        main_state.output_manager.note_on(OutRangeNote[0], OutRangeNote[1], OutRangeNote[2]);
                        counter += 1;
                        //self.outrange_note_list.pop();
                    }
                    for counter in 0..counter {
                        self.outrange_note_list.pop();    
                    }  범위 밖 노트 여러 개가 동시에 눌려야 하는 경우... 아직 미완성*/
                    self.outrange_note = false;
                    main_state.output_manager.note_on(self.outrange_note_list[0], self.outrange_note_list[1], self.outrange_note_list[2]);
                    self.outrange_note_list.pop();
                    self.outrange_note_list.pop();
                    self.outrange_note_list.pop();   //범위 밖 노트가 한번에 하나만 눌리면 되는 경우
                }
                
            } else {
                self.input_pressed_keys[event.1 as usize - 36] = false;
                main_state.output_manager.note_off(0, event.1)
            }
        }
        //log::warn!("required_notes: {:?}", self.required_notes.lock().unwrap().len());
        
        if self.required_notes.lock().unwrap().len() == 0 && self.waiting_for_note == true {
            //main_state.output_manager.note_on(self.outrange_note_list[0], self.outrange_note_list[1], self.outrange_note_list[2]);
            //self.outrange_note_list.pop();
            self.waiting_for_note = false;
            //self.outrange_note = false;
            timer.resume();
        }  
    }

    fn require_note(&mut self, timer: &mut Timer, n: &MidiNote) {
        //if n.note >= 21  + 15 && n.note <= 108 - 12 && n.ch != 9 && n.ch != 8{
            //let time_check = time::Instant::now();   //editted for test
            self.required_notes
                .lock()
                .unwrap()
                .insert(n.note , n.clone());
            self.waiting_for_note = true;
            //log::warn!("time elapse: {:?}", time_check); //editted for test
            timer.pause();
        //}
    }

    fn clear(&mut self) {
        self.required_notes.lock().unwrap().clear();
    }
}

struct Toast {
    start_time: std::time::Instant,
    inner_draw: Box<dyn Fn(&mut Target)>,
}

impl Toast {
    fn new(draw: impl Fn(&mut Target) + 'static) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            inner_draw: Box::new(draw),
        }
    }

    fn draw(&mut self, target: &mut Target) -> bool {
        let time = self.start_time.elapsed().as_secs();
        
        if time < 1 {
            (*self.inner_draw)(target);

            true
        } else {
            false
        }
    }
}
