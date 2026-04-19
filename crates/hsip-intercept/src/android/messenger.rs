use crate::{MessagingEvent, Result};

pub async fn open_messenger_activity(_hint: Option<String>) -> Result<()> {
    unimplemented!("Android messenger activity — JNI bridge not yet implemented")
}

pub fn extract_recipient_from_view(_event: &MessagingEvent) -> Result<String> {
    unimplemented!("Android recipient extraction — JNI bridge not yet implemented")
}
