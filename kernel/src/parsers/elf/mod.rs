mod header;


enum ParseError {
    InvalidMagic,
    InvalidClass,
    InvalidDataEncoding,
    InvalidVersion,
    IncompleteData,
    Other,
}
