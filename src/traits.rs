// src/traits.rs
//! Defines traits for use elsewhere

pub trait Permit<T, E>
where
    T: Default,
{
    fn permit<F>(self, f: F) -> Result<T, E>
    where
        F: FnOnce(&E) -> bool;
}

impl<E> Permit<(), E> for Result<(), E> {
    /// Lazy error handling
    /// Lets you permit a specific error for `Result<(), E>`
    /// Note that this does *not* work for `T`
    ///
    /// **Example:**
    /// ```rust
    /// // Attempt to create a directory, but permit the case where it already exists
    /// if let Err(e) =
    ///     std::fs::create_dir("/tmp/dir").permit(|e| e.kind() == std::io::ErrorKind::AlreadyExists)
    /// {
    ///     // If a different error exists, handle it as usual
    ///     eprintln!("Failed to create /tmp/dir: {e}")
    /// }
    /// ```
    ///
    /// You can chain this
    fn permit<F>(self, f: F) -> Result<(), E>
    where
        F: FnOnce(&E) -> bool,
    {
        match self {
            | Ok(()) => Ok(()),             // if result is ok, return Ok(())
            | Err(ref e) if f(e) => Ok(()), // permit the error and return Ok(())
            | Err(e) => Err(e),             // return the original error if not permitted
        }
    }
}
