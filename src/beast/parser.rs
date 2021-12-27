use crate::beast::*;

use log::debug;

use nom::branch::*;
use nom::bytes::streaming::*;
use nom::combinator::*;
use nom::error::*;
use nom::multi::*;
use nom::sequence::*;
use nom::IResult;

const MODE_AC_LENGTH: usize = 2;
const MODE_S_SHORT_LENGTH: usize = 7;
const MODE_S_LONG_LENGTH: usize = 14;

#[derive(Default)]
pub struct Parser {}

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    pub fn parse<'a>(&'a self, input: &'a [u8]) -> IResult<&'a [u8], Message> {
        parse(input)
    }
}

fn parse<'a>(input: &'a [u8]) -> IResult<&'a [u8], Message> {
    let (input, (_, (message_length, timestamp, signal_level))) = many_till(
        take(1usize),
        tuple((
            map(
                preceded(tag(b"\x1a"), alt((tag(b"1"), tag(b"2"), tag(b"3")))),
                |message_format: &[u8]| match message_format {
                    b"1" => MODE_AC_LENGTH,
                    b"2" => MODE_S_SHORT_LENGTH,
                    b"3" => MODE_S_LONG_LENGTH,
                    v => panic!("unknown BEAST message format: {:?}", v),
                },
            ),
            map(take_while_m_n(6, 6, |_| true), |ts: &[u8]| {
                ts.iter().fold(0u32, |ts, c| (ts << 8) | *c as u32)
            }),
            map(take(1usize), |signal: &[u8]| {
                let signal = signal[0] as f64 / 255.0;
                signal * signal
            }),
        )),
    )(input)?;

    debug!("message_length: {}", message_length);
    debug!("timestamp: {}", timestamp);
    debug!("signal_level: {}", signal_level);

    let (input, message) = preceded(
        many0(tag("\x1a")),
        take_while_m_n(message_length, message_length, |_| true),
    )(input)?;

    debug!("message: {:x?}", message);

    if message_length == MODE_AC_LENGTH {
        return Ok((
            input,
            Message {
                timestamp,
                signal_level,
                data: Data::Unsupported(message.to_vec()),
            },
        ));
    }

    let message = mode_s(timestamp, signal_level, message).unwrap().1;

    Ok((input, message))
}

fn mode_s<'a>(timestamp: u32, signal_level: f64, input: &'a [u8]) -> IResult<&'a [u8], Message> {
    use nom::bits::bits;
    use nom::bits::complete::take;

    map(
        bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(take(5usize)),
        |downlink_format: u8| {
            debug!("DF={}", downlink_format);

            let data = match downlink_format {
                0 => parse_df_0(input),
                4 => parse_df_4(input),
                5 => parse_df_5(input),
                11 => parse_df_11(input),
                16 => parse_df_16(input),
                17 => parse_df_17(input),
                _ => Data::Unsupported(input.to_vec()),
            };

            Message {
                timestamp,
                signal_level,
                data,
            }
        },
    )(input)
}

pub(crate) fn parse_df_0(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(preceded::<_, u8, _, _, _, _>(
        take(5usize),
        map(
            tuple((
                map(take(1usize), vertical_status),
                map(take(1usize), cross_link),
                preceded::<_, u8, _, _, _, _>(take(1usize), map(take(3usize), sensitivity_level)),
                preceded::<_, u8, _, _, _, _>(take(2usize), map(take(4usize), reply_information)),
                preceded::<_, u8, _, _, _, _>(take(2usize), map(take(13usize), altitude_code)),
            )),
            |(vertical_status, cross_link, sensitivity_level, reply_information, altitude)| {
                Data::ACASSurveillanceReply(ACASSurveillanceReply {
                    vertical_status,
                    cross_link,
                    sensitivity_level,
                    reply_information,
                    altitude,
                })
            },
        ),
    ))(input)
    .unwrap()
    .1
}

pub(crate) fn parse_df_4(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(map(
        tuple((
            preceded::<_, u8, _, _, _, _>(take(5usize), map(take(3usize), flight_status)),
            take(5usize),
            take(6usize),
            map(take(13usize), altitude_code),
        )),
        |(flight_status, downlink_request, utility_message, altitude)| {
            Data::AltitudeReply(AltitudeReply {
                flight_status,
                downlink_request,
                utility_message,
                altitude,
            })
        },
    ))(input)
    .unwrap()
    .1
}

pub(crate) fn parse_df_5(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(map(
        tuple((
            preceded::<_, u8, _, _, _, _>(take(5usize), map(take(3usize), flight_status)),
            take(5usize),
            take(6usize),
            map(take(13usize), ident),
        )),
        |(flight_status, downlink_request, utility_message, id)| {
            Data::SurveillanceReply(SurveillanceReply {
                flight_status,
                downlink_request,
                utility_message,
                id,
            })
        },
    ))(input)
    .unwrap()
    .1
}

pub(crate) fn parse_df_11(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(preceded::<_, u8, _, _, _, _>(
        take(5usize),
        map(
            tuple((
                map(take(3usize), capability),
                map(take(24usize), address_announced),
                take(24usize),
            )),
            |(capability, icao, parity)| {
                Data::AllCallReply(AllCallReply {
                    capability,
                    icao,
                    parity,
                })
            },
        ),
    ))(input)
    .unwrap()
    .1
}

pub(crate) fn parse_df_16(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(preceded::<_, u8, _, _, _, _>(
        take(5usize),
        map(
            tuple((
                map(take(1usize), vertical_status),
                preceded::<_, u8, _, _, _, _>(take(2usize), map(take(3usize), sensitivity_level)),
                preceded::<_, u8, _, _, _, _>(take(2usize), map(take(4usize), reply_information)),
                preceded::<_, u8, _, _, _, _>(take(2usize), map(take(13usize), altitude_code)),
                map(take(56usize), |message: u64| {
                    message.to_be_bytes()[1..].to_vec()
                }),
            )),
            |(vertical_status, sensitivity_level, reply_information, altitude, message)| {
                Data::ACASCoordinationReply(ACASCoordinationReply {
                    vertical_status,
                    sensitivity_level,
                    reply_information,
                    altitude,
                    message,
                })
            },
        ),
    ))(input)
    .unwrap()
    .1
}

pub(crate) fn parse_df_17(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(preceded::<_, u8, _, _, _, _>(
        take(5usize),
        map(
            tuple((
                map(take(3usize), capability),
                map(take(24usize), address_announced),
                map(take(56usize), message),
            )),
            |(capability, icao, message)| {
                Data::ExtendedSquitter(ExtendedSquitter {
                    capability,
                    icao,
                    message,
                })
            },
        ),
    ))(input)
    .unwrap()
    .1
}

// AA
fn address_announced(aa: u32) -> String {
    format!("{:X}", aa)
}

const ONES_PATTERN: [(u16, u16); 3] = [(0x10, 0x7), (0x20, 0x3), (0x40, 0x1)];

const FIVES_PATTERN: [(u16, u16); 8] = [
    (0x0002, 0xff),
    (0x0004, 0x7f),
    (0x1000, 0x3f),
    (0x2000, 0x1f),
    (0x4000, 0x0f),
    (0x0100, 0x07),
    (0x0200, 0x03),
    (0x0400, 0x01),
];

const Q_BIT: u16 = 0x10;

// AC
fn altitude_code(ac: u16) -> Altitude {
    if 0 == ac {
        return Altitude::Invalid;
    }

    match 0x40 == (0x40 & ac) {
        true => {
            // meters
            Altitude::Meters(0)
        }
        false => {
            // feet
            match Q_BIT == (Q_BIT & ac) {
                true => {
                    // × 25 foot
                    let feet: i32 = (((0x1f80 & ac) >> 2) | ((0x20 & ac) >> 1) | (0xf & ac)).into();

                    Altitude::Feet((feet * 25) - 1000)
                }
                false => {
                    let mode_a = decode(ID_PATTERN, ac);
                    let index = (mode_a & 0x7)
                        | ((mode_a & 0x70) >> 1)
                        | ((mode_a & 0x700) >> 2)
                        | ((mode_a & 0x7000) >> 3);

                    match index {
                        0..=4095 => {
                            if (index & 0x8889) != 0 || (index & 0xf0) == 0 {
                                return Altitude::Invalid;
                            }

                            let ones = ONES_PATTERN
                                .iter()
                                .map(|(in_bit, xor_bits)| {
                                    if *in_bit == index & in_bit {
                                        *xor_bits
                                    } else {
                                        0
                                    }
                                })
                                .fold(0, |acc, v| acc ^ v);

                            let fives = FIVES_PATTERN
                                .iter()
                                .map(|(in_bit, xor_bits)| {
                                    if *in_bit == index & in_bit {
                                        *xor_bits
                                    } else {
                                        0
                                    }
                                })
                                .fold(0, |acc, v| acc ^ v);

                            let ones = if fives & 1 == 1 { 6 - ones } else { ones };

                            Altitude::Feet((fives * 5 + ones - 13).into())
                        }
                        4096.. => Altitude::Invalid,
                    }
                }
            }
        }
    }
}

// CA
fn capability(ca: u8) -> u8 {
    ca
}

// CC
fn cross_link(cc: u8) -> CrossLink {
    match cc {
        0 => CrossLink::Unsupported,
        1 => CrossLink::Supported,
        _ => unreachable!("Impossible cross-link capability {}", cc), // one bit field
    }
}

// FS
fn flight_status(fs: u8) -> FlightStatus {
    match fs {
        0 => FlightStatus {
            alert: false,
            spi: false,
            status: VerticalStatus::Airborne,
        },
        1 => FlightStatus {
            alert: false,
            spi: false,
            status: VerticalStatus::Ground,
        },
        2 => FlightStatus {
            alert: true,
            spi: false,
            status: VerticalStatus::Airborne,
        },
        3 => FlightStatus {
            alert: true,
            spi: false,
            status: VerticalStatus::Ground,
        },
        4 => FlightStatus {
            alert: true,
            spi: true,
            status: VerticalStatus::Either,
        },
        5 => FlightStatus {
            alert: false,
            spi: true,
            status: VerticalStatus::Either,
        },
        6 => unreachable!("FS=0b110 is reserved"),
        7 => unreachable!("FS=0b111 is not assigned"),
        _ => unreachable!("FS={} is larger than 3 bits", fs),
    }
}

// ID
fn ident(id: u16) -> u16 {
    decode(ID_PATTERN, id)
}

// ME
fn message(me: u64) -> ADSBMessage {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let input = me.to_be_bytes();
    let input = &input[1..];

    let (_, type_code) =
        bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(take(5usize))(input).unwrap();

    match type_code {
        1..=4 => aircraft_identification(input, type_code),
        //5..=8 => unimplemented!("surface_position"),
        9..=18 => airborne_position(input),
        19 => velocity(input),
        //20..=22 => unimplemented!("airborne_position"),
        28 => aircraft_status(input),
        29 => target_state(input),
        //31 => unimplemented!("operational_status"),
        _ => ADSBMessage::Unsupported(input.to_vec()),
    }
}

// RI
fn reply_information(ri: u8) -> ReplyInformation {
    match ri {
        0 => ReplyInformation::Inoperative,
        2 => ReplyInformation::ACASInhibited,
        3 => ReplyInformation::ACASVerticalOnly,
        4 => ReplyInformation::ACASVerticalAndHorizontal,
        8 => ReplyInformation::NoMaximumAirspeed,
        9 => ReplyInformation::MaximumAirspeedUnder(75),
        10 => ReplyInformation::MaximumAirspeedBetween(75, 150),
        11 => ReplyInformation::MaximumAirspeedBetween(150, 300),
        12 => ReplyInformation::MaximumAirspeedBetween(300, 600),
        13 => ReplyInformation::MaximumAirspeedBetween(600, 1200),
        14 => ReplyInformation::MaximumAirspeedOver(1200),
        _ => ReplyInformation::Unsupported(ri),
    }
}

// SL
fn sensitivity_level(sl: u8) -> SensitivityLevel {
    match sl {
        0 => SensitivityLevel::Inoperative,
        1..=7 => SensitivityLevel::Operative(sl),
        _ => unreachable!("Impossible sensitivity level {}", sl),
    }
}

// VS
fn vertical_status(vs: u8) -> VerticalStatus {
    match vs {
        0 => VerticalStatus::Either,
        1 => VerticalStatus::Ground,
        _ => unreachable!("Impossible vertical status {}", vs), // one bit field
    }
}

const ID_PATTERN: [(u16, u16); 12] = [
    (0x1000, 0x0010),
    (0x0800, 0x1000),
    (0x0400, 0x0020),
    (0x0200, 0x2000),
    (0x0100, 0x0040),
    (0x0080, 0x4000),
    (0x0020, 0x0100),
    (0x0010, 0x0001),
    (0x0008, 0x0200),
    (0x0004, 0x0002),
    (0x0002, 0x0400),
    (0x0001, 0x0004),
];

fn decode(pattern: [(u16, u16); 12], encoded: u16) -> u16 {
    pattern
        .iter()
        .map(|(in_bit, out_bit)| {
            if *in_bit == encoded & in_bit {
                *out_bit
            } else {
                0
            }
        })
        .fold(0, |acc, v| acc | v)
}

fn airborne_position(input: &[u8]) -> ADSBMessage {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let (_, message) = bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(map(
        tuple((
            preceded::<_, u8, _, _, _, _>(take(5usize), map(take(2usize), surveillance_status)),
            map(take(1usize), |saf: u8| saf == 1),
            map(take(12usize), altitude_code),
            map(take(1usize), |t: u8| t == 1),
            map(take(1usize), cpr_format),
            take(17usize),
            take(17usize),
        )),
        |(
            surveillance_status,
            single_antenna,
            altitude,
            utc_synchronized,
            cpr_format,
            cpr_latitude,
            cpr_longitude,
        )| {
            ADSBMessage::AirbornePosition(AirbornePosition {
                surveillance_status,
                single_antenna,
                altitude,
                utc_synchronized,
                cpr_format,
                cpr_latitude,
                cpr_longitude,
            })
        },
    ))(input)
    .unwrap();

    message
}

fn aircraft_identification(input: &[u8], type_code: u8) -> ADSBMessage {
    use nom::bits::bits;
    use nom::bits::complete::take;

    bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(preceded::<_, u8, _, _, _, _>(
        take(5usize),
        map(
            tuple((
                map(take(3usize), |category| {
                    aircraft_category(type_code, category)
                }),
                map(
                    tuple((
                        map(take(6usize), call_sign_character),
                        map(take(6usize), call_sign_character),
                        map(take(6usize), call_sign_character),
                        map(take(6usize), call_sign_character),
                        map(take(6usize), call_sign_character),
                        map(take(6usize), call_sign_character),
                        map(take(6usize), call_sign_character),
                        map(take(6usize), call_sign_character),
                    )),
                    |(c0, c1, c2, c3, c4, c5, c6, c7)| {
                        let mut call_sign = String::with_capacity(8);
                        call_sign.push(c0);
                        call_sign.push(c1);
                        call_sign.push(c2);
                        call_sign.push(c3);
                        call_sign.push(c4);
                        call_sign.push(c5);
                        call_sign.push(c6);
                        call_sign.push(c7);
                        call_sign
                    },
                ),
            )),
            |(category, call_sign)| {
                ADSBMessage::AircraftIdentification(AircraftIdentification {
                    category,
                    call_sign,
                })
            },
        ),
    ))(input)
    .unwrap()
    .1
}

fn aircraft_status(input: &[u8]) -> ADSBMessage {
    use nom::bits::bits;
    use nom::bits::complete::take;

    bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(preceded::<_, u8, _, _, _, _>(
        take(5usize),
        map(
            tuple((take(3usize), take(3usize), map(take(13usize), ident))),
            |(sub_type, emergency, squawk)| {
                ADSBMessage::AircraftStatus(AircraftStatus::new(sub_type, emergency, squawk))
            },
        ),
    ))(input)
    .unwrap()
    .1
}

fn aircraft_category(type_code: u8, category: u8) -> AircraftCategory {
    if 1 == type_code {
        unreachable!("reserved aircraft category for type code {}", type_code);
    }

    if 0 == category {
        return AircraftCategory::None;
    }

    match type_code {
        2 => match category {
            1 => AircraftCategory::SurfaceEmergencyVehicle,
            3 => AircraftCategory::SurfaceServiceVehicle,
            4..=7 => AircraftCategory::GroundObstruction,
            _ => unreachable!(
                "unknown aircraft category {} for type code {}",
                category, type_code
            ),
        },
        3 => match category {
            1 => AircraftCategory::Glider,
            2 => AircraftCategory::LighterThanAir,
            3 => AircraftCategory::Parachutist,
            4 => AircraftCategory::Ultralight,
            5 => unreachable!(
                "reserved aircraft category {} for type code {}",
                category, type_code
            ),
            6 => AircraftCategory::UnmannedAerialVehicle,
            7 => AircraftCategory::SpaceVehicle,
            _ => unreachable!(
                "impossible aircraft category {} for type code {}",
                category, type_code
            ),
        },
        4 => match category {
            1 => AircraftCategory::Light,
            2 => AircraftCategory::Medium1,
            3 => AircraftCategory::Medium2,
            4 => AircraftCategory::HighVortexAircraft,
            5 => AircraftCategory::Heavy,
            6 => AircraftCategory::HighPerformance,
            7 => AircraftCategory::Rotorcraft,
            _ => unreachable!(
                "impossible aircraft category {} for type code {}",
                category, type_code
            ),
        },
        _ => unreachable!("impossible type code {}", type_code),
    }
}

fn call_sign_character(c: u32) -> char {
    match c {
        1..=26 => char::from_u32(c + 64).unwrap(),
        32 => ' ',
        48..=57 => char::from_u32(c).unwrap(),
        _ => unreachable!("invalid call sign character {}", c),
    }
}

fn cpr_format(f: u8) -> CPRFormat {
    match f {
        0 => CPRFormat::Even,
        1 => CPRFormat::Odd,
        _ => unreachable!("invalid CPR format {}", f),
    }
}

fn surveillance_status(ss: u8) -> SurveillanceStatus {
    match ss {
        0 => SurveillanceStatus::NoCondition,
        1 => SurveillanceStatus::PermanentAlert,
        2 => SurveillanceStatus::TemporaryAlert,
        3 => SurveillanceStatus::SPICondition,
        _ => unreachable!("impossible surveillance status {}", ss),
    }
}

fn target_state(input: &[u8]) -> ADSBMessage {
    use nom::bits::bits;
    use nom::bits::complete::tag;
    use nom::bits::complete::take;

    bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(preceded::<_, u8, _, _, _, _>(
        take(5usize),
        map(
            alt((
                preceded::<_, u8, _, _, _, _>(
                    tag(0, 2usize), // sub-type 0
                    map(
                        tuple((
                            take(2usize),
                            take(1usize),
                            take(1usize),
                            take(2usize),
                            take(2usize),
                            take(10usize),
                            take(2usize),
                            take(9usize),
                            take(1usize),
                            take(2usize),
                            take(4usize),
                            take(1usize),
                            take(2usize),
                            preceded::<_, u8, _, _, _, _>(take(5usize), take(2usize)),
                            take(3usize),
                        )),
                        |_: (u8, u8, u8, u8, u8, u16, u8, u8, u8, u8, u8, u8, u8, u8, u8)| {
                            TargetStateType::SubType0(TargetState0 {})
                        },
                    ),
                ),
                preceded::<_, u8, _, _, _, _>(
                    tag(1, 2usize), // sub-type 1
                    map(
                        tuple((
                            map(take(1usize), sil_supplement),
                            map(take(1usize), |fms: u8| fms == 1),
                            map(take(10usize), altitude_setting),
                            map(take(9usize), barometer_setting),
                            map(tuple((take(1usize), take(9usize))), heading_setting),
                            map(take(3usize), nac_position),
                            map(take(1usize), nic_barometric),
                            map(take(2usize), sil),
                            alt((
                                map(tag(0, 1usize), |_| {
                                    (false, None, None, None, None, None, None)
                                }),
                                tuple((
                                    map(tag(1, 1usize), |_| true),
                                    map(take(1usize), |b: u8| match b {
                                        0 => Some(false),
                                        1 => Some(true),
                                        _ => unreachable!("impossible autopilot flag {}", b),
                                    }),
                                    map(take(1usize), |b: u8| match b {
                                        0 => Some(false),
                                        1 => Some(true),
                                        _ => unreachable!("impossible VNAV flag {}", b),
                                    }),
                                    map(take(1usize), |b: u8| match b {
                                        0 => Some(false),
                                        1 => Some(true),
                                        _ => {
                                            unreachable!("impossible altitude hold flag {}", b)
                                        }
                                    }),
                                    map(take(1usize), |b: u8| match b {
                                        0 => Some(false),
                                        1 => Some(true),
                                        _ => {
                                            unreachable!("impossible autopilot approach flag {}", b)
                                        }
                                    }),
                                    map(take(1usize), |b: u8| match b {
                                        0 => Some(false),
                                        1 => Some(true),
                                        _ => unreachable!("impossible TCAS flag {}", b),
                                    }),
                                    map(take(1usize), |b: u8| match b {
                                        0 => Some(false),
                                        1 => Some(true),
                                        _ => unreachable!("impossible LNAV flag {}", b),
                                    }),
                                )),
                            )),
                        )),
                        |(
                            sil_supplement,
                            fms,
                            altitude_setting,
                            barometer_setting,
                            heading_setting,
                            nac_position,
                            nic_barometric,
                            sil,
                            (
                                known_source,
                                autopilot,
                                vnav,
                                altitude_hold,
                                autopilot_approach,
                                tcas,
                                lnav,
                            ),
                        )| {
                            let altitude_source = match known_source {
                                false => AltitudeSource::Unknown,
                                true => match fms {
                                    false => AltitudeSource::MCPFCU,
                                    true => AltitudeSource::FMS,
                                },
                            };

                            TargetStateType::SubType1(TargetState1 {
                                sil_supplement,
                                altitude_source,
                                altitude_setting,
                                barometer_setting,
                                heading_setting,
                                nac_position,
                                nic_barometric,
                                sil,
                                autopilot,
                                vnav,
                                altitude_hold,
                                autopilot_approach,
                                tcas,
                                lnav,
                            })
                        },
                    ),
                ),
            )),
            ADSBMessage::TargetState,
        ),
    ))(input)
    .unwrap()
    .1
}

fn altitude_setting(altitude_setting: u32) -> AltitudeSetting {
    match altitude_setting {
        0 => AltitudeSetting::None,
        _ => AltitudeSetting::Feet(altitude_setting * 32),
    }
}

fn barometer_setting(barometer_setting: u16) -> BarometerSetting {
    match barometer_setting {
        0 => BarometerSetting::None,
        _ => BarometerSetting::MilliBar(800.0 + (barometer_setting as f64 - 1.0) * 0.8),
    }
}

fn heading_setting(input: (u8, u16)) -> HeadingSetting {
    match input.0 {
        0 => HeadingSetting::None,
        1 => HeadingSetting::MagneticOrTrue((input.1 as f64 * 180.0) / 256.0),
        _ => unreachable!("impossible heading setting {}", input.0),
    }
}

fn nac_position(nac_p: u8) -> u8 {
    nac_p
}

fn nic_barometric(nic_b: u8) -> u8 {
    nic_b
}

fn sil(sil: u8) -> SourceIntegrityLevel {
    match sil {
        0 => SourceIntegrityLevel::Unknown,
        1 => SourceIntegrityLevel::PerThousand,
        2 => SourceIntegrityLevel::PerHundredThousand,
        3 => SourceIntegrityLevel::PerTenMillion,
        _ => unreachable!("impossible source integrity level {}", sil),
    }
}

fn sil_supplement(sil_supplement: u8) -> SourceIntegrityLevelSupplement {
    match sil_supplement {
        0 => SourceIntegrityLevelSupplement::PerHour,
        1 => SourceIntegrityLevelSupplement::PerSample,
        _ => unreachable!(
            "impossible source integrity level supplement {}",
            sil_supplement
        ),
    }
}

fn velocity(input: &[u8]) -> ADSBMessage {
    use nom::bits::bits;
    use nom::bits::complete::take;

    bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(preceded::<_, u8, _, _, _, _>(
        take(5usize),
        map(
            tuple((
                take(3usize), // sub_type
                map(take(1usize), |ic: u8| ic == 1),
                map(take(1usize), |ifr: u8| ifr == 1),
                take(3usize),  // navigation uncertainty
                take(22usize), // velocity
                map(
                    // vertical rate
                    tuple((
                        take(1usize),
                        map(take(1usize), |vr_sign: u8| vr_sign == 1),
                        take(9usize),
                    )),
                    vertical_rate,
                ),
                map(
                    // altitude difference
                    tuple((
                        preceded::<_, u8, _, _, _, _>(
                            take(2usize),
                            map(take(1usize), |diff_sign: u8| diff_sign == 1),
                        ),
                        take(7usize),
                    )),
                    altitude_difference,
                ),
            )),
            |(
                sub_type,
                intent_change,
                ifr_capability,
                navigation_uncertainty,
                velocity,
                vertical_rate,
                altitude_difference,
            )| {
                ADSBMessage::Velocity(Velocity::new(
                    sub_type,
                    intent_change,
                    ifr_capability,
                    navigation_uncertainty,
                    velocity,
                    vertical_rate,
                    altitude_difference,
                ))
            },
        ),
    ))(input)
    .unwrap()
    .1
}

fn altitude_difference(input: (bool, u8)) -> AltitudeDifference {
    let below = input.0;
    let difference: i16 = input.1.into();

    let sign = match below {
        true => -1,
        false => 1,
    };

    match difference {
        0 => AltitudeDifference::NoInformation,
        _ => AltitudeDifference::Feet(sign * 25 * difference),
    }
}

fn vertical_rate(input: (u8, bool, u16)) -> VerticalRate {
    let source = input.0;
    let down = input.1;
    let rate: i32 = input.2.into();

    match rate {
        0 => VerticalRate::NoInformation,
        _ => {
            let sign = match down {
                true => -1,
                false => 1,
            };

            let rate = sign * 64 * (rate - 1);

            match source {
                0 => VerticalRate::FeetPerMinute(VerticalRateSource::GNSS(rate)),
                1 => VerticalRate::FeetPerMinute(VerticalRateSource::Barometer(rate)),
                _ => unreachable!("Impossible vertical rate source {}", source),
            }
        }
    }
}
