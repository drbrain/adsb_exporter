use nom::combinator::*;
use nom::error::*;
use nom::sequence::*;

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
    AirbornePosition(AirbornePosition),
    Velocity(Velocity),
}

#[derive(Debug, PartialEq)]
pub struct AirbornePosition {
    pub surveillance_status: SurveillanceStatus,
    pub single_antenna: bool,
    pub altitude: Altitude,
    pub utc_synchronized: bool,
    pub cpr_format: CPRFormat,
    pub cpr_latitude: u32,
    pub cpr_longitude: u32,
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
pub struct Airspeed {
    pub supersonic_aircraft: bool,
    pub magnetic_heading_available: bool,
    pub magnetic_heading: u16,
    pub airspeed_type: AirspeedType,
    pub airspeed: u16,
}

#[derive(Debug, PartialEq)]
pub enum AirspeedType {
    Indicated,
    True,
}

#[derive(Debug, PartialEq)]
pub enum Altitude {
    Invalid,
    Feet(i32),
    Meters(i32),
}

#[derive(Debug, PartialEq)]
pub enum AltitudeDifference {
    NoInformation,
    Feet(i16),
}

#[derive(Debug, PartialEq)]
pub struct AltitudeReply {
    pub flight_status: FlightStatus,
    pub downlink_request: u8,
    pub utility_message: u8,
    pub altitude: Altitude,
}

#[derive(Debug, PartialEq)]
pub enum CPRFormat {
    Even,
    Odd,
}

#[derive(Debug, PartialEq)]
pub enum CrossLink {
    Unsupported,
    Supported,
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
pub enum EastWestDirection {
    WestToEast,
    EastToWest,
}

#[derive(Debug, PartialEq)]
pub struct ExtendedSquitter {
    pub capability: u8,
    pub icao: String,
    pub message: ADSBMessage,
}

#[derive(Debug, PartialEq)]
pub struct FlightStatus {
    pub alert: bool,
    pub spi: bool,
    pub status: AircraftStatus,
}

#[derive(Debug, PartialEq)]
pub struct GroundVelocity {
    pub supersonic_aircraft: bool,
    pub east_west_direction: EastWestDirection,
    pub east_west_velocity: u16,
    pub north_south_direction: NorthSouthDirection,
    pub north_south_velocity: u16,
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
pub enum NorthSouthDirection {
    SouthToNorth,
    NorthToSouth,
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

#[derive(Debug, PartialEq)]
pub enum SurveillanceStatus {
    NoCondition,
    PermanentAlert,
    TemporaryAlert,
    SPICondition,
}

#[derive(Debug, PartialEq)]
pub struct Velocity {
    pub intent_change: bool,
    pub ifr_capability: bool,
    pub navigation_uncertainty: u8, // TODO
    pub velocity: VelocityType,
    pub vertical_rate_source: VerticalRateSource,
    pub vertical_rate: VerticalRate,
    pub altitude_difference: AltitudeDifference,
}

impl Velocity {
    pub fn new(
        sub_type: u8,
        intent_change: bool,
        ifr_capability: bool,
        navigation_uncertainty: u8,
        velocity: u32,
        vertical_rate_source: VerticalRateSource,
        vertical_rate: VerticalRate,
        altitude_difference: AltitudeDifference,
    ) -> Velocity {
        let velocity = match sub_type {
            1 => velocity_ground(false, velocity),
            2 => velocity_ground(true, velocity),
            3 => velocity_airborne(false, velocity),
            4 => velocity_airborne(true, velocity),
            _ => unreachable!("impossible velocity sub-type {}", sub_type),
        };

        Velocity {
            intent_change,
            ifr_capability,
            navigation_uncertainty,
            velocity,
            vertical_rate_source,
            vertical_rate,
            altitude_difference,
        }
    }
}

fn velocity_airborne(supersonic_aircraft: bool, velocity: u32) -> VelocityType {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let input = velocity.to_be_bytes();

    let velocity =
        bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(preceded::<_, u16, _, _, _, _>(
            take(10usize),
            map(
                tuple((
                    map(take(1usize), |m: u8| m == 1),
                    take(10usize),
                    map(take(1usize), |t: u8| match t {
                        0 => AirspeedType::Indicated,
                        1 => AirspeedType::True,
                        _ => unreachable!("impossible airspeed type {}", t),
                    }),
                    take(10usize),
                )),
                |(magnetic_heading_available, magnetic_heading, airspeed_type, airspeed)| {
                    Airspeed {
                        supersonic_aircraft,
                        magnetic_heading_available,
                        magnetic_heading,
                        airspeed_type,
                        airspeed,
                    }
                },
            ),
        ))(&input)
        .unwrap()
        .1;

    VelocityType::Airborne(velocity)
}

fn velocity_ground(supersonic_aircraft: bool, velocity: u32) -> VelocityType {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let input = velocity.to_be_bytes();

    let velocity =
        bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(preceded::<_, u16, _, _, _, _>(
            take(10usize),
            map(
                tuple((
                    map(take(1usize), |d: u8| match d {
                        0 => EastWestDirection::WestToEast,
                        1 => EastWestDirection::EastToWest,
                        _ => unreachable!("impossible east-west direction {}", d),
                    }),
                    take(10usize),
                    map(take(1usize), |d: u8| match d {
                        0 => NorthSouthDirection::SouthToNorth,
                        1 => NorthSouthDirection::NorthToSouth,
                        _ => unreachable!("impossible north-south direction {}", d),
                    }),
                    take(10usize),
                )),
                |(
                    east_west_direction,
                    east_west_velocity,
                    north_south_direction,
                    north_south_velocity,
                )| {
                    GroundVelocity {
                        supersonic_aircraft,
                        east_west_direction,
                        east_west_velocity,
                        north_south_direction,
                        north_south_velocity,
                    }
                },
            ),
        ))(&input)
        .unwrap()
        .1;

    VelocityType::Ground(velocity)
}

#[derive(Debug, PartialEq)]
pub enum VelocityType {
    Airborne(Airspeed),
    Ground(GroundVelocity),
}

#[derive(Debug, PartialEq)]
pub enum VerticalRate {
    NoInformation,
    FeetPerMinute(i32),
}

#[derive(Debug, PartialEq)]
pub enum VerticalRateSource {
    GNSS,
    Barometer,
}
