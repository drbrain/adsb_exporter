#[derive(Debug, PartialEq)]
pub struct ACASSurveillanceReply {
    pub vertical_status: AircraftStatus,
    pub cross_link: CrossLink,
    pub sensitivity_level: SensitivityLevel,
    pub reply_information: ReplyInformation,
    pub altitude: Altitude,
}

#[derive(Debug, PartialEq)]
pub enum ADSBMessage {
    AircraftIdentification(AircraftIdentification),
}

#[derive(Debug, PartialEq)]
pub enum AircraftCategory {
    None,
    SurfaceEmergencyVehicle,
    SurfaceServiceVehicle,
    GroundObstruction,
    Glider,
    LighterThanAir,
    Parachutist,
    Ultralight,
    UnmannedAerialVehicle,
    SpaceVehicle,
    Light,
    Medium1,
    Medium2,
    HighVortexAircraft,
    Heavy,
    HighPerformance,
    Rotorcraft,
}

#[derive(Debug, PartialEq)]
pub struct AircraftIdentification {
    pub category: AircraftCategory,
    pub call_sign: String,
}

#[derive(Debug, PartialEq)]
pub enum AircraftStatus {
    OnGround,
    Airborne,
    Either,
}

#[derive(Debug, PartialEq)]
pub enum Altitude {
    Invalid,
    Feet(i32),
    Meters(i32),
}

#[derive(Debug, PartialEq)]
pub struct AltitudeReply {
    pub flight_status: FlightStatus,
    pub downlink_request: u8,
    pub utility_message: u8,
    pub altitude: Altitude,
}

#[derive(Debug, PartialEq)]
pub enum CrossLink {
    Unsupported,
    Supported,
}

#[derive(Debug, PartialEq)]
pub struct FlightStatus {
    pub alert: bool,
    pub spi: bool,
    pub status: AircraftStatus,
}

#[derive(Debug, PartialEq)]
pub enum Data {
    ACASSurveillanceReply(ACASSurveillanceReply),
    AltitudeReply(AltitudeReply),
    ExtendedSquitter(ExtendedSquitter),
    SurveillanceReply(SurveillanceReply),
    Unsupported(Vec<u8>),
}

#[derive(Debug, PartialEq)]
pub struct ExtendedSquitter {
    pub capability: u8,
    pub icao: String,
    pub message: ADSBMessage,
}

#[derive(Debug, PartialEq)]
pub enum Message {
    ModeS(ModeS),
    Unsupported(String),
}

#[derive(Debug, PartialEq)]
pub struct ModeS {
    pub timestamp: u32,
    pub signal_level: f64,
    pub data: Data,
}

#[derive(Debug, PartialEq)]
pub enum ReplyInformation {
    Inoperative,
    ACASInhibited,
    ACASVerticalOnly,
    ACASVerticalAndHorizontal,
}

#[derive(Debug, PartialEq)]
pub enum SensitivityLevel {
    Inoperative,
    Operative(u8),
}

#[derive(Debug, PartialEq)]
pub struct SurveillanceReply {
    pub flight_status: FlightStatus,
    pub downlink_request: u8,
    pub utility_message: u8,
    pub id: u16,
}
