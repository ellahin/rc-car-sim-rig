use common_data::server::data::telementry::Telementry;

pub enum UDPSessionState {
    UnAuthed,
    Authed,
}
pub struct UDPSession {
    stream: Vec<u8>,
    username: Option<String>,
    telementry: Option<Telementry>,
    state: UDPSessionState,
}
