//! The `errors` module defines the common error types.

use core::fmt;

/// `Error` provides an enumeration of all possible errors reported by Cauldron.
#[derive(Debug)]
pub enum Error {
    /// File Not Found On Path Provided.
    FileNotFound(String),
    /// The ebook contained malformed data and could not be parsed.
    ParseError(String),
    /// An unsupported format is passed.
    Unsupported(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::FileNotFound(ref path) => write!(f, "File Not Found on path: {}", path),
            &Error::ParseError(ref msg) => write!(f, "Malformed epub encountered: {}", msg),
            &Error::Unsupported(ref msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match *self {
            Error::FileNotFound(_) => None,
            Error::ParseError(_) => None,
            Error::Unsupported(_) => None,
        }
    }
}

impl From<roxmltree::Error> for Error {
    fn from(err: roxmltree::Error) -> Self {
        Error::ParseError(format!("Invalid Epub Format: {}", err.to_string()))
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(err: zip::result::ZipError) -> Self {
        Error::ParseError(format!("Invalid Epub Format: {}", err.to_string()))
    }
}

/// function to create file not found error
pub fn to_fnf_error(file_path: String) -> Error {
    Error::FileNotFound(file_path)
}

/// function to create an parse error
pub fn to_parse_error() -> Error {
    Error::ParseError("Invalid Epub Format".to_string())
}


// /// function to create an unsupported codec error.
// pub fn unsupported_error<T>(msg: &'static str) -> Result<T> {
//     Err(Error::Unsupported(msg))
// }
