use bevy::prelude::*;

#[derive(Resource, Default, Debug)]
pub struct InstanceManager<T: Default> {
    free_list: Vec<u32>,
    data: Vec<T>,
    dirty: bool,
}

impl<T: Default> InstanceManager<T> {
    pub fn alloc_index(&mut self) -> u32 {
        self.dirty = true;
        if let Some(index) = self.free_list.pop() {
            index
        } else {
            let index = self.data.len() as u32;
            self.data.push(T::default());
            index
        }
    }

    pub fn dealloc_index(&mut self, index: u32) {
        self.free_list.push(index);
    }

    pub fn get_mut(&mut self, index: u32) -> &mut T {
        self.dirty = true;
        &mut self.data[index as usize]
    }

    pub fn get_buffer_data(&self) -> &[T] {
        &self.data
    }

    pub fn reset_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}
