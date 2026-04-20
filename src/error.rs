use std::fmt;

#[derive(Debug)]
pub enum FileParseError{
    MissingData,
    MissingPoint,
    InvalidDataType(String),
    DataOutOfBounds(usize),
    MissingCoordinate,
    FailedLineParse(usize, Box<FileParseError>),
    IoError(std::io::Error),
    NotSupportedFileFormat(Option<String>),
}

impl From<std::io::Error> for FileParseError {
    fn from(err: std::io::Error) -> Self {
        FileParseError::IoError(err)
    }
}