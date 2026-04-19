use crate::{InterceptConfig, InterceptOverlay, Result};

pub struct AndroidOverlay;

impl AndroidOverlay {
    pub fn new(_config: &InterceptConfig) -> Result<Box<dyn InterceptOverlay>> {
        unimplemented!("Android overlay — WindowManager bridge not yet implemented")
    }
}
