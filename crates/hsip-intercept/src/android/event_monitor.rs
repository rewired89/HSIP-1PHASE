use crate::{EventMonitor, InterceptConfig, Result};
use tokio::sync::mpsc;

pub struct AndroidEventMonitor;

impl AndroidEventMonitor {
    pub fn new(
        _tx: mpsc::Sender<crate::MessagingEvent>,
        _config: &InterceptConfig,
    ) -> Result<Box<dyn EventMonitor>> {
        unimplemented!("Android event monitor — JNI bridge not yet implemented")
    }
}
