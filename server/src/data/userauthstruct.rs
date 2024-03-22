use std::time::SystemTime;

pub struct UserAuth {
    pub code: String,
    pub created: SystemTime,
}
