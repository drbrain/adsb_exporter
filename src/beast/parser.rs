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
        bits::<_, _, Error<(&[u8], usize)>, Error<&[u8]>, _>(take(4usize))(input).unwrap();

    dbg!(message_type);

    let data = match message_type {
        0 => Message::ModeS(ModeS {
            timestamp,
            signal_level,
            data: parse_df_0(input),
        }),
        5 => Message::ModeS(ModeS {
            timestamp,
            signal_level,
            data: parse_df_5(input),
        }),
        _ => panic!("message type {} not supported", message_type),
    };

    Ok((input, data))
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

fn aa(input: &[u8], message_type: u8) -> Option<u32> {
    match message_type {
        11 | 17 | 18 => Some(input[1..3].iter().fold(0u32, |ts, c| (ts << 8) | *c as u32)),
        _ => None,
    }
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
            match 0x10 == (0x10 & ac) {
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
        0 => FlightStatus::Uncertain,
        1 => FlightStatus::Ground,
        2 => FlightStatus::Uncertain, // also, alert?
        3 => FlightStatus::Ground,    // also, alert?
        4 => FlightStatus::Uncertain, // also alert? spi?
        5 => FlightStatus::Uncertain, // also spi?
        _ => unreachable!("BUG: Unknown flight status {}", fs),
    }
}

// ID
fn ident(id: u16) -> u16 {
    decode(ID_PATTERN, id)
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
fn vertical_status(vs: u8) -> FlightStatus {
    match vs {
        0 => FlightStatus::Uncertain,
        1 => FlightStatus::Ground,
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
