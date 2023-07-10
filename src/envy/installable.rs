use anyhow::Result;

pub trait Installable {
    fn install(&self) -> Result<String>;
}
