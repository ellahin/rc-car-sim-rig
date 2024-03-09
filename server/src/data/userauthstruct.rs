use std::time::SystemTime;

pub struct UserAuth {
    pub code: [char; 8],
    pub created: SystemTime,
}
