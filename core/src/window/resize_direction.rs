/// Defines the orientation that a window resize will be performed.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ResizeDirection {
    /// East
    East,
    /// North
    North,
    /// North East
    NorthEast,
    /// North West
    NorthWest,
    /// South
    South,
    /// South East
    SouthEast,
    /// South West
    SouthWest,
    /// West
    West,
}
