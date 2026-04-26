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

impl fmt::Display for FileParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileParseError::MissingData => write!(f, "Missing data to parse"),
            FileParseError::MissingPoint => write!(f, "Missing point to parse"),
            FileParseError::InvalidDataType(s) => write!(f, "Failed to parse data for '{}', invalid type", s),
            FileParseError::DataOutOfBounds(u) => write!(f, "Index '{}' is out of bounds", u),
            FileParseError::MissingCoordinate => write!(f, "Missing coordinate to parse"),
            FileParseError::FailedLineParse(u, error) => write!(f, "Failed to parse line '{}' due to error '{}'", u, *error),
            FileParseError::IoError(error) => write!(f, "Failed due to Io Error '{}'", error),
            
            FileParseError::NotSupportedFileFormat(option) => {
                if let Some(format) = option {
                    write!(f, "Could not parse file, the '{}' format is not supported yet", format)
                } else {
                    write!(f, "The input you selected is not a file and could not be parsed")
                }
            },
        }
    }
}


#[derive(Debug)]
pub enum SaveAndLoadError{
    IoError(std::io::Error),
    NotSupportedFileFormat(Option<String>),
}

impl From<std::io::Error> for SaveAndLoadError {
    fn from(err: std::io::Error) -> Self {
        SaveAndLoadError::IoError(err)
    }
}

impl fmt::Display for SaveAndLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveAndLoadError::IoError(error) => write!(f, "Failed due to Io Error '{}'", error),
            
            SaveAndLoadError::NotSupportedFileFormat(option) => {
                if let Some(format) = option {
                    write!(f, "Could not parse file, the '{}' format is not supported yet", format)
                } else {
                    write!(f, "The input you selected is not a file and could not be parsed")
                }
            },
        }
    }
}