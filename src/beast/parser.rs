use crate::beast::*;

use log::debug;

use nom::bytes::streaming::*;
use nom::combinator::*;
use nom::error::*;
use nom::sequence::*;
use nom::IResult;

type VE<'a> = VerboseError<&'a [u8]>;

const MODE_AC_LENGTH: usize = 2;
const MODE_S_SHORT_LENGTH: usize = 7;
const MODE_S_LONG_LENGTH: usize = 14;

pub struct Parser {}

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    pub fn parse<'a>(&'a self, input: &'a [u8]) -> IResult<&'a [u8], Message, VE> {
        parse::<VE>(input)
    }
}

fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Message, E> {
    let (input, _) = tag(b"\x1a")(input)?;

    let (input, message_format) = take(1usize)(input)?;
    debug!(
        "message_format: {}",
        std::str::from_utf8(&message_format).unwrap()
    );

    let message_length = match message_format {
        b"1" => MODE_AC_LENGTH,
        b"2" => MODE_S_SHORT_LENGTH,
        b"3" => MODE_S_LONG_LENGTH,
        v => panic!("unsupported: {:?}", v),
    };
    debug!("message_length: {}", message_length);

    let (input, ts) = take_while_m_n(6, 6, |_| true)(input)?;
    let timestamp = ts.iter().fold(0u32, |ts, c| (ts << 8) | *c as u32);
    debug!("timestamp: {}", timestamp);

    let (input, signal) = take(1usize)(input)?;
    let signal = signal[0] as f64 / 255.0;
    let signal = signal * signal;
    debug!("signal: {}", signal);

    let (input, message) = take_while_m_n(message_length, message_length, |_| true)(input)?;

    debug!("message: {:?}", message);

    if message_length == MODE_AC_LENGTH {
        return Ok((
            input,
            Message::Unsupported("Mode A/C not supported".to_string()),
        ));
    }

    let (_, message) = mode_s(timestamp, signal, message).unwrap();

    Ok((input, message))
}

fn mode_s(timestamp: u32, signal_level: f64, input: &[u8]) -> IResult<&[u8], Message> {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let (_, message_type): (_, u8) =
        bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(take(5usize))(input).unwrap();

    let data = match message_type {
        0 => parse_df_0(input),
        4 => parse_df_4(input),
        5 => parse_df_5(input),
        17 => parse_df_17(input),
        _ => panic!("message type {} not supported", message_type),
    };

    let mode_s = Message::ModeS(ModeS {
        timestamp,
        signal_level,
        data,
    });

    Ok((input, mode_s))
}

pub(crate) fn parse_df_0(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let (_, reply) = bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(map(
        tuple((
            preceded::<_, u8, _, _, _, _>(take(5usize), map(take(1usize), vertical_status)),
            map(take(1usize), cross_link),
            preceded::<_, u8, _, _, _, _>(take(1usize), map(take(3usize), sensitivity_level)),
            preceded::<_, u8, _, _, _, _>(take(2usize), map(take(4usize), reply_information)),
            preceded::<_, u8, _, _, _, _>(take(2usize), map(take(13usize), altitude_code)),
        )),
        |(vertical_status, cross_link, sensitivity_level, reply_information, altitude)| {
            ACASSurveillanceReply {
                vertical_status,
                cross_link,
                sensitivity_level,
                reply_information,
                altitude,
            }
        },
    ))(input)
    .unwrap();

    Data::ACASSurveillanceReply(reply)
}

pub(crate) fn parse_df_4(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let (_, reply) = bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(map(
        tuple((
            preceded::<_, u8, _, _, _, _>(take(5usize), map(take(3usize), flight_status)),
            take(5usize),
            take(6usize),
            map(take(13usize), altitude_code),
        )),
        |(flight_status, downlink_request, utility_message, altitude)| AltitudeReply {
            flight_status,
            downlink_request,
            utility_message,
            altitude,
        },
    ))(input)
    .unwrap();

    Data::AltitudeReply(reply)
}

pub(crate) fn parse_df_5(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let (_, reply) = bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(map(
        tuple((
            preceded::<_, u8, _, _, _, _>(take(5usize), map(take(3usize), flight_status)),
            take(5usize),
            take(6usize),
            map(take(13usize), ident),
        )),
        |(flight_status, downlink_request, utility_message, id)| SurveillanceReply {
            flight_status,
            downlink_request,
            utility_message,
            id,
        },
    ))(input)
    .unwrap();

    Data::SurveillanceReply(reply)
}

pub(crate) fn parse_df_17(input: &[u8]) -> Data {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let (_, message) = bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(map(
        tuple((
            preceded::<_, u8, _, _, _, _>(take(5usize), map(take(3usize), capability)),
            map(take(24usize), address_announced),
            map(take(56usize), message),
        )),
        |(capability, icao, message)| ExtendedSquitter {
            capability,
            icao,
            message,
        },
    ))(input)
    .unwrap();

    Data::ExtendedSquitter(message)
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
            status: AircraftStatus::Airborne,
        },
        1 => FlightStatus {
            alert: false,
            spi: false,
            status: AircraftStatus::OnGround,
        },
        2 => FlightStatus {
            alert: true,
            spi: false,
            status: AircraftStatus::Airborne,
        },
        3 => FlightStatus {
            alert: true,
            spi: false,
            status: AircraftStatus::OnGround,
        },
        4 => FlightStatus {
            alert: true,
            spi: true,
            status: AircraftStatus::Either,
        },
        5 => FlightStatus {
            alert: false,
            spi: true,
            status: AircraftStatus::Either,
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

    dbg!(me);

    let input = me.to_be_bytes();

    let (_, type_code) =
        bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(take(5usize))(&input[1..]).unwrap();

    dbg!(type_code);

    match type_code {
        1..=4 => {
            let (_, category) =
                bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(
                    preceded::<_, u8, _, _, _, _>(
                        take(5usize),
                        map(take(3usize), |category| {
                            aircraft_category(type_code, category)
                        }),
                    ),
                )(&input[1..])
                .unwrap();

            let call_sign = aircraft_identification(&input[2..]);

            ADSBMessage::AircraftIdentification(AircraftIdentification {
                category,
                call_sign,
            })
        }
        5..=8 => unimplemented!("surface_position"),
        9..=18 => unimplemented!("airborne_position"),
        19 => unimplemented!("airborne_velocity"),
        20..=22 => unimplemented!("airborne_position"),
        28 => unimplemented!("aircraft_status"),
        29 => unimplemented!("target_states"),
        31 => unimplemented!("operational_status"),
        _ => unreachable!("Unsupported type code {}", type_code),
    }
}

// RI
fn reply_information(ri: u8) -> ReplyInformation {
    match ri {
        0 => ReplyInformation::Inoperative,
        2 => ReplyInformation::ACASInhibited,
        3 => ReplyInformation::ACASVerticalOnly,
        4 => ReplyInformation::ACASVerticalAndHorizontal,
        _ => unreachable!("Impossible reply information {}", ri),
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
fn vertical_status(vs: u8) -> AircraftStatus {
    match vs {
        0 => AircraftStatus::Either,
        1 => AircraftStatus::OnGround,
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

// aircraft_category
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

// aircraft_identification
fn aircraft_identification(id: &[u8]) -> String {
    use nom::bits::bits;
    use nom::bits::complete::take;

    let (_, call_sign) = bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(map(
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
    ))(id)
    .unwrap();

    call_sign
}

fn call_sign_character(c: u32) -> char {
    match c {
        1..=26 => char::from_u32(c + 64).unwrap(),
        32 => ' ',
        48..=57 => char::from_u32(c).unwrap(),
        _ => unreachable!("invalid call sign character {}", c),
    }
}
