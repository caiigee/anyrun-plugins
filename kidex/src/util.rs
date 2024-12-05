pub enum IndexAction {
    Open,
    CopyPath,
    Back,
}

impl From<u64> for IndexAction {
    fn from(value: u64) -> Self {
        match value {
            0 => Self::Open,
            1 => Self::CopyPath,
            2 => Self::Back,
            _ => unreachable!(),
        }
    }
}
