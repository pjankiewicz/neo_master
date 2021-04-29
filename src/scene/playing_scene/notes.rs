use super::notes_pipeline::{NoteInstance, NotesPipeline};
use crate::target::Target;
use crate::wgpu_jumpstart::Color;
use crate::TransformUniform;
use crate::Uniform;

pub struct Notes {
    notes_pipeline: NotesPipeline,

    pub instance_buffer: Vec<NoteInstance>,
}

impl Notes {
    pub fn new(target: &mut Target, keys: &[super::keyboard::Key]) -> Self {
        let notes_pipeline = NotesPipeline::new(target, target.state.midi_file.as_ref().unwrap());
        let mut notes = Self { notes_pipeline, instance_buffer: Vec::new()};
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

                self.instance_buffer.push(NoteInstance {
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
    pub fn recolor(&mut self, target: &mut Target, note_color: usize) {
        
        let mut instances = Vec::new();
        let i = self.instance_buffer.len();
        //log::warn!("length: {:?}", i);
        let color_schema = &target.state.config.color_schema;
        let qwe = &target.state.config.color_schema.len();
        log::warn!("length: {:?}", qwe);
        for x in 0..i {
            //let color_schema = &target.state.config.color_schema;
            //let qwe = &target.state.config.color_schema.len();
            //log::warn!("length: {:?}", qwe);
            /*if note_color != 0 {
                let color = &color_schema[note_color - 1 ];
            } else {
                let color = &color_schema[i - 1 ];
            }*/
            let mut color_index = i;
            if note_color != 0 {
                color_index = note_color - 1;
            } else {
                color_index = i - 1;
            }
            let color = &color_schema[color_index];

            if self.instance_buffer[x].color[0] as u8 == color.dark.0 {
                let color = &color_schema[note_color];
                let color = color.dark;
                let color: Color = color.into();
                self.instance_buffer[x].color = color.into_linear_rgb();
                instances.push(self.instance_buffer[x]);

            } else if self.instance_buffer[x].color[0] as u8 == color.base.0 {
                let color = &color_schema[note_color];
                let color = color.base;
                let color: Color = color.into();
                self.instance_buffer[x].color = color.into_linear_rgb();
                instances.push(self.instance_buffer[x]);
            }
            
            
        }
        //instances = self.instance_buffer;
        self.notes_pipeline
            .update_instance_buffer(&mut target.gpu, instances);   //아마 이 부분이 내가 원하는대로 작동 안 할거임
        
    }
    pub fn render<'rpass>(
        &'rpass mut self,
        transform_uniform: &'rpass Uniform<TransformUniform>,
        render_pass: &mut wgpu::RenderPass<'rpass>,
    ) {
        self.notes_pipeline.render(transform_uniform, render_pass);
    }
}
