use super::notes_pipeline::{NoteInstance, NotesPipeline};
use crate::target::Target;
use crate::wgpu_jumpstart::Color;
use crate::TransformUniform;
use crate::Uniform;

pub struct Notes {
    notes_pipeline: NotesPipeline,

    pub note_hand: Vec<usize>,
}

impl Notes {
    pub fn new(target: &mut Target, keys: &[super::keyboard::Key]) -> Self {
        let notes_pipeline = NotesPipeline::new(target, target.state.midi_file.as_ref().unwrap());
        let mut notes = Self { notes_pipeline, note_hand: Vec::new() };
        notes.resize(target, keys);
        notes
    }
    pub fn resize(&mut self, target: &mut Target, keys: &[super::keyboard::Key]) {
        let midi = &target.state.midi_file.as_ref().unwrap();

        let (window_w, window_h) = {
            let winit::dpi::LogicalSize { width, height } = target.window.state.logical_size;
            (width, height)
        };

        let mut instances = Vec::new();

        let mut longer_than_88 = false;
        for note in midi.merged_track.notes.iter() {
            if note.note >= 21 + 15 && note.note <= 108 - 12 - 3 && note.ch != 9 && note.ch != 8 /*&& note.ch != 7 && note.ch != 6 */{
                let key = &keys[note.note as usize - 21 - 15];
                let ar = window_w / window_h;

                let color_schema = &target.state.config.color_schema;

                let color = &color_schema[note.track_id % color_schema.len()];
                self.note_hand.push(note.track_id % color_schema.len());  //recolor()에서 노트의 왼손 오른손 구분을 위해 note_hand에 저장
                
                let color = if key.is_black { color.dark } else { color.base };
                let color: Color = color.into();

                let h = if note.duration >= 0.1 {
                    note.duration
                } else {
                    0.1 
                };

                instances.push(NoteInstance {
                    position: [key.x, note.start],
                    size: [key.w - 1.0, h - 0.01], // h - 0.01 to make a litle gap between successive notes
                    color: color.into_linear_rgb(),
                    radius: 4.0 * ar,
                });
                
            } else {
                longer_than_88 = true;
            }
        }

        if longer_than_88 {
            log::warn!("Midi Wider Than 88 Keys!");
        }

        self.notes_pipeline
            .update_instance_buffer(&mut target.gpu, instances);
    }
    pub fn update(&mut self, target: &mut Target, time: f32) {
        self.notes_pipeline.update_time(&mut target.gpu, time);
    }
    //F1 키를 누르면 노트 색이 바뀌도록 하는 함수.. "settings.ron" 에서 default을 변경해줘야 적용됨, 혹은 ron 파일을 지우고 config를 수정하면 됨
    pub fn recolor(&mut self, target: &mut Target, keys: &[super::keyboard::Key]) {
        let midi = &target.state.midi_file.as_ref().unwrap();

        let (window_w, window_h) = {
            let winit::dpi::LogicalSize { width, height } = target.window.state.logical_size;
            (width, height)
        };

        let mut instances = Vec::new();
        let mut note_count: usize = 0;
        for note in midi.merged_track.notes.iter() {
            if note.note >= 21 + 15 && note.note <= 108 - 12 - 3 && note.ch != 9 && note.ch != 8 /*&& note.ch != 7 && note.ch != 6 */{
                let key = &keys[note.note as usize - 21 - 15];
                let ar = window_w / window_h;

                let color_schema = &target.state.config.color_schema;

                self.note_hand[note_count] += 2;

                if self.note_hand[note_count] == color_schema.len() {
                    self.note_hand[note_count] = 0;
                } else if self.note_hand[note_count] > color_schema.len() {
                    self.note_hand[note_count] = 1;   
                }

                let color = &color_schema[self.note_hand[note_count]];
                let color = if key.is_black { color.dark } else { color.base };
                let color: Color = color.into();

                let h = if note.duration >= 0.1 {
                    note.duration
                } else {
                    0.1 
                };

                instances.push(NoteInstance {
                    position: [key.x, note.start],
                    size: [key.w - 1.0, h - 0.01], // h - 0.01 to make a litle gap between successive notes
                    color: color.into_linear_rgb(),
                    radius: 4.0 * ar,
                });

                note_count += 1;

            }
        }

        self.notes_pipeline
            .update_instance_buffer(&mut target.gpu, instances);
    }
    pub fn render<'rpass>(
        &'rpass mut self,
        transform_uniform: &'rpass Uniform<TransformUniform>,
        render_pass: &mut wgpu::RenderPass<'rpass>,
    ) {
        self.notes_pipeline.render(transform_uniform, render_pass);
    }
}
