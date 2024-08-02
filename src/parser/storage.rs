//! Storage for the parser to expand macros call into.

/// This storage is used by the parser to store the expaned content of a macro call. It is only
/// used when user-defined macros are used. Otherwise, this storage is not used and is _zero-overhead_.
///
/// [`Storage`] needs to exist because Rust does not allow self-referencial types. When Rust does
/// (hopefully) gain support for self-referencial types, this storage will be removed.
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

