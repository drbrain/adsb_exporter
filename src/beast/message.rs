#[derive(Debug, PartialEq)]
pub enum FlightStatus {
    Airborne,
    Ground,
    Uncertain,
}

#[derive(Debug)]
pub enum Message {
    ModeS(ModeS),
    Unsupported(String),
}

#[derive(Debug)]
pub struct ModeS {
    pub timestamp: u32,
    pub signal_level: f64,
    pub data: Data,
}

#[derive(Debug, PartialEq)]
pub enum Data {
    SurveillanceReply(SurveillanceReply),
}

#[derive(Debug, PartialEq)]
pub struct SurveillanceReply {
    pub flight_status: FlightStatus,
    pub downlink_request: u8,
    pub utility_message: u8,
    pub id: u16,
}

#[derive(Debug)]
pub enum Source {
    Adsb,
    ModeS,
}
