//! Various constants, for use in various places. Mostly for rendering, but there are a few for other reasons.

/// Constants with gameplay implications, albeit potentially minor.
pub mod gameplay {
    /// The maximum length of a username. Mostly used to compute minimum size.
    pub const MAX_USERNAME: usize = 10;
}

/// Constants specifically relating to how things render.
pub mod graphics {
    /// How many rows the header takes up. Used by apps for relative positioning.
    pub const HEADER_HEIGHT: usize = 1;
}
