use skyline::nn;

#[repr(C)]
pub struct File_NX {
    vtable: *const (),
    unk1: *const (),
    unk2: u32,
    pub is_open: u32,
    pub file_handle: *mut nn::fs::FileHandle,
    pub unk3: u32,
    pub position: u64,
    pub filename_fixedstring: [u8; 516],
    unk4: u32,
}

impl File_NX {
    pub fn read(&mut self, buffer: &mut [u8]) -> u64 {
        unsafe {
            let mut out = 0;
			nn::fs::ReadFile1(&mut out, *self.file_handle, self.position as i64, buffer.as_mut_ptr(), buffer.len() as u64);
			self.position += out;
            out
        }
    }
    pub fn set_position(&mut self, position: usize) {
        self.position = position as u64;
    }
}

#[repr(C)]
pub struct SemaphorePlatform {
    inner: *mut (*const (), *mut nn::os::SemaphoreType)
}

impl SemaphorePlatform {
    pub fn acquire(&mut self) {
        unsafe { nn::os::AcquireSemaphore((unsafe { *self.inner }).1) };
    }
    pub fn release(&mut self) {
        unsafe { nn::os::ReleaseSemaphore((unsafe { *self.inner }).1) };
    }
}

#[repr(C)]
pub struct SystemEventPlatform {
    inner: *mut *mut nn::os::EventType,
}

impl SystemEventPlatform {
    pub fn wait(&mut self) {
        unsafe { nn::os::WaitEvent(unsafe { *self.inner }) };
    }
    pub fn signal(&mut self) {
        unsafe { nn::os::SignalEvent(unsafe { *self.inner }) };
    }
}