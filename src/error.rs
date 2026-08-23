use std::path::{Path, PathBuf};

/// What can go wrong while reading or writing a notebook.
///
/// The point of naming these is that the caller can tell them apart. A missing
/// vault is a setup mistake worth explaining; a failed write is a data-loss risk
/// worth retrying and shouting about. `Box<dyn Error>` collapsed them into one
/// opaque string, which is why every failure used to surface as the same flat
/// "Failed to..." line.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("vault path does not exist: {0}")]
    VaultMissing(PathBuf),

    #[error("vault path is not a directory: {0}")]
    VaultNotDirectory(PathBuf),

    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not move {from} to {to}: {source}")]
    Rename {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: std::io::Error,
    },

}

/// Attach the path to an io error so the message can name the file.
pub trait IoResultExt<T> {
    fn write_ctx(self, path: &Path) -> Result<T, StorageError>;
    fn create_dir_ctx(self, path: &Path) -> Result<T, StorageError>;
}

impl<T> IoResultExt<T> for Result<T, std::io::Error> {
    fn write_ctx(self, path: &Path) -> Result<T, StorageError> {
        self.map_err(|source| StorageError::Write { path: path.to_path_buf(), source })
    }
    fn create_dir_ctx(self, path: &Path) -> Result<T, StorageError> {
        self.map_err(|source| StorageError::CreateDir { path: path.to_path_buf(), source })
    }
}
