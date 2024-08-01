#[derive(Default)]
pub struct Storage(pub(super) bumpalo::Bump);

impl Storage {
    /// Create a new storage for the parser.
    pub fn new() -> Self {
        Default::default()
    }

    /// Reset the storage's memory.
    ///
    /// It is recommended to call this method after each parsing operation to free up memory. This
    /// is more efficient than dropping the storage and creating a new one.
    pub fn reset(&mut self) {
        self.0.reset();
    }
}

